//! 库存物品仓储操作。
//!
//! 本模块属于 `core` 持久化层，封装 `stock_items` 的创建、查询、更新、软删除、物品详情库存快照和物品审计写入。
//! service 不应直接拼接库存物品表查询。

use std::collections::HashMap;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DbErr, EntityTrait,
    QueryFilter, QueryResult, Set, Statement, TransactionTrait, Value,
};
use serde_json::json;

use super::{
    common::insert_audit_event_on_connection, search, CatalogAttributeRecord, CatalogSort,
    CatalogStockFilter, CreateStockItem, ItemAttributeInput, ItemCatalogCountsRecord,
    ItemCatalogCriteria, ItemCatalogFieldFilter, ItemCatalogPage, ItemCatalogRecord,
    ItemInventoryRecord, ItemOptionCriteria, ItemOptionRecord, Page, StockItemBatchRecord,
    StockItemListRecord, StockItemLocationRecord, StockRepository, UpdateStockItem,
};
use crate::persistence::{
    entity::{item_attribute, item_attribute_definition, stock_item},
    repository::{time::sqlite_now, validation::validate_repository_input},
};

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建未删除库存物品，并在同一事务内写入可选审计事件。
    pub(crate) async fn create_item(
        &self,
        input: CreateStockItem,
        audit_user_id: Option<i64>,
    ) -> Result<stock_item::Model, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        ensure_item_image_available(&transaction, input.image_file_id, input.image_owner_user_id)
            .await?;
        let now = sqlite_now(&transaction).await?;
        let active_model = stock_item::ActiveModel {
            name: Set(input.name),
            sku: Set(input.sku),
            category_id: Set(input.category_id),
            attribute_template_id: Set(input.attribute_template_id),
            image_file_id: Set(input.image_file_id),
            unit: Set(input.unit),
            description: Set(input.description),
            default_price: Set(input.default_price),
            reorder_point: Set(input.reorder_point),
            created_at: Set(now.clone()),
            updated_at: Set(now),
            deleted_at: Set(None),
            ..Default::default()
        };
        let result = stock_item::Entity::insert(active_model)
            .exec(&transaction)
            .await?;

        let item = find_active_item_by_id_on_connection(&transaction, result.last_insert_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created stock item".to_owned()))?;
        replace_item_attributes_on_connection(&transaction, item.id, &input.attributes).await?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "item",
                Some(item.id),
                "created",
                Some(item_created_details(&item)),
            )
            .await?;
        }
        transaction.commit().await?;

        Ok(item)
    }

    /// 查询未软删除物品详情；软删除记录不会返回给业务服务层。
    pub(crate) async fn find_active_item_by_id(
        &self,
        id: i64,
    ) -> Result<Option<stock_item::Model>, DbErr> {
        find_active_item_by_id_on_connection(self.database, id).await
    }

    /// 查询编辑器需要的物品基础资料和全部固有属性，不加载库存数据。
    pub(crate) async fn find_active_item_editor_by_id(
        &self,
        id: i64,
    ) -> Result<Option<StockItemListRecord>, DbErr> {
        let Some(item) = self.find_active_item_by_id(id).await? else {
            return Ok(None);
        };
        let attributes = list_item_attributes_on_connection(self.database, id).await?;
        Ok(Some(StockItemListRecord { item, attributes }))
    }

    /// 判断私有属性定义是否属于指定物品，供服务层在整体替换属性前校验定义归属。
    pub(crate) async fn item_owns_attribute_definition(
        &self,
        item_id: i64,
        definition_id: i64,
    ) -> Result<bool, DbErr> {
        Ok(item_attribute_definition::Entity::find_by_id(definition_id)
            .filter(item_attribute_definition::Column::OwnerItemId.eq(item_id))
            .one(self.database)
            .await?
            .is_some())
    }

    /// 查询可用于结构化筛选的共享物品属性定义 ID。
    pub(crate) async fn searchable_item_attribute_definition_ids(
        &self,
        definition_ids: &[i64],
    ) -> Result<Vec<i64>, DbErr> {
        if definition_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(item_attribute_definition::Entity::find()
            .filter(item_attribute_definition::Column::Id.is_in(definition_ids.iter().copied()))
            .filter(item_attribute_definition::Column::OwnerItemId.is_null())
            .filter(item_attribute_definition::Column::Searchable.eq(1))
            .all(self.database)
            .await?
            .into_iter()
            .map(|definition| definition.id)
            .collect())
    }

    /// 查询指定 SKU 是否已有其他未软删除物品占用。
    pub(crate) async fn active_sku_exists_except(
        &self,
        sku: &str,
        except_id: Option<i64>,
    ) -> Result<bool, DbErr> {
        let mut query = stock_item::Entity::find()
            .filter(stock_item::Column::DeletedAt.is_null())
            .filter(stock_item::Column::Sku.eq(sku));
        if let Some(except_id) = except_id {
            query = query.filter(stock_item::Column::Id.ne(except_id));
        }

        Ok(query.one(self.database).await?.is_some())
    }

    /// 查询物品目录库存视图；库存聚合、状态筛选、计数和排序均在数据库内完成。
    pub(crate) async fn list_item_catalog(
        &self,
        input: ItemCatalogCriteria,
    ) -> Result<ItemCatalogPage, DbErr> {
        let search_like = input
            .search
            .as_ref()
            .map(|search| format!("%{}%", search.to_lowercase()));
        let (base_sql, base_values) = item_catalog_base_query(
            search_like.as_deref(),
            input.category_id,
            input.attribute_template_id,
            &input.field_filters,
        );
        let counts = self
            .query_item_catalog_counts(&base_sql, base_values.clone())
            .await?;
        let filter_sql = catalog_filter_sql(input.stock_filter);
        let total_sql = format!("SELECT COUNT(*) AS count FROM ({base_sql}) catalog {filter_sql}");
        let total_row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                total_sql,
                base_values.clone(),
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("item catalog count".to_owned()))?;
        let total = u64::try_from(total_row.try_get::<i64>("", "count")?).unwrap_or(0);

        let limit = input.page_size as i64;
        let offset = ((input.page.saturating_sub(1)) * input.page_size) as i64;
        let mut page_values = base_values;
        page_values.push(limit.into());
        page_values.push(offset.into());
        let page_sql = format!(
            "SELECT * FROM ({base_sql}) catalog {filter_sql} {} LIMIT ? OFFSET ?",
            catalog_order_sql(input.sort)
        );
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                page_sql,
                page_values,
            ))
            .await?;
        let mut items = rows
            .into_iter()
            .map(|row| {
                Ok(ItemCatalogRecord {
                    item: stock_item_from_query_row(&row)?,
                    category_name: row.try_get("", "category_name")?,
                    current_quantity: row.try_get("", "current_quantity")?,
                    inventory_value: row.try_get("", "inventory_value")?,
                    location_count: u64::try_from(row.try_get::<i64>("", "location_count")?)
                        .unwrap_or(0),
                    batch_count: u64::try_from(row.try_get::<i64>("", "batch_count")?).unwrap_or(0),
                    stock_state: row.try_get("", "stock_state")?,
                    catalog_attributes: Vec::new(),
                })
            })
            .collect::<Result<Vec<_>, DbErr>>()?;
        let attributes = self
            .query_catalog_attributes(&items.iter().map(|item| item.item.id).collect::<Vec<_>>())
            .await?;
        for item in &mut items {
            item.catalog_attributes = attributes.get(&item.item.id).cloned().unwrap_or_default();
        }

        Ok(ItemCatalogPage {
            items,
            total,
            counts,
        })
    }

    /// 查询业务选择器使用的轻量物品分页，不执行库存聚合。
    pub(crate) async fn list_item_options(
        &self,
        input: ItemOptionCriteria,
    ) -> Result<Page<ItemOptionRecord>, DbErr> {
        let search_like = input
            .search
            .as_ref()
            .map(|search| format!("%{}%", search.to_lowercase()));
        let (base_sql, values) = item_option_base_query(
            search_like.as_deref(),
            input.category_id,
            input.attribute_template_id,
        );
        let count_row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!("SELECT COUNT(*) AS count FROM ({base_sql}) options"),
                values.clone(),
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("item option count".to_owned()))?;
        let total = u64::try_from(count_row.try_get::<i64>("", "count")?).unwrap_or(0);
        let mut page_values = values;
        page_values.push((input.page_size as i64).into());
        page_values.push((((input.page.saturating_sub(1)) * input.page_size) as i64).into());
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!(
                    "SELECT * FROM ({base_sql}) options ORDER BY lower(name), id LIMIT ? OFFSET ?"
                ),
                page_values,
            ))
            .await?;
        let items = rows
            .into_iter()
            .map(|row| {
                Ok(ItemOptionRecord {
                    id: row.try_get("", "id")?,
                    name: row.try_get("", "name")?,
                    sku: row.try_get("", "sku")?,
                    category_id: row.try_get("", "category_id")?,
                    category_name: row.try_get("", "category_name")?,
                    attribute_template_id: row.try_get("", "attribute_template_id")?,
                    recommended_inbound_template_id: row
                        .try_get("", "recommended_inbound_template_id")?,
                    recommended_inbound_template_available: row
                        .try_get::<i64>("", "recommended_inbound_template_available")?
                        != 0,
                    image_file_id: row.try_get("", "image_file_id")?,
                    unit: row.try_get("", "unit")?,
                })
            })
            .collect::<Result<Vec<_>, DbErr>>()?;
        Ok(Page { items, total })
    }

    /// 查询已有物品的库存摘要和库位分布，不加载批次明细。
    pub(crate) async fn find_item_inventory_by_id(
        &self,
        id: i64,
    ) -> Result<Option<ItemInventoryRecord>, DbErr> {
        let Some(item) = self.find_active_item_by_id(id).await? else {
            return Ok(None);
        };
        let (current_quantity, inventory_value) = self.query_item_stock_summary(id).await?;
        let batch_count = self.query_item_batch_count(id).await?;
        let stock_state = stock_state_code(current_quantity, item.reorder_point).to_owned();
        let locations = self.query_item_stock_locations(id).await?;
        Ok(Some(ItemInventoryRecord {
            item,
            current_quantity,
            inventory_value,
            stock_state,
            batch_count,
            locations,
        }))
    }

    /// 分页查询单个物品当前仍有余额的批次。
    pub(crate) async fn list_item_stock_batches(
        &self,
        item_id: i64,
        page: u64,
        page_size: u64,
    ) -> Result<Page<StockItemBatchRecord>, DbErr> {
        let total = self.query_item_batch_count(item_id).await?;
        let limit = page_size as i64;
        let offset = ((page.saturating_sub(1)) * page_size) as i64;
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT batches.id, batches.batch_no, batches.location_id,
                       locations.name AS location_name,
                       batches.initial_quantity, batches.remaining_quantity, batches.unit_cost,
                       batches.remaining_quantity * batches.unit_cost AS value,
                       batches.received_at, batches.expires_at
                FROM stock_batches batches
                JOIN stock_locations locations ON locations.id = batches.location_id
                WHERE batches.item_id = ? AND batches.remaining_quantity > 0
                ORDER BY batches.expires_at IS NULL ASC, batches.expires_at ASC,
                         batches.received_at ASC, batches.id ASC
                LIMIT ? OFFSET ?
                "#,
                [item_id.into(), limit.into(), offset.into()],
            ))
            .await?;
        let items = rows
            .into_iter()
            .map(stock_batch_from_query_row)
            .collect::<Result<Vec<_>, DbErr>>()?;
        Ok(Page { items, total })
    }

    /// 更新未软删除物品，并在同一事务内记录字段前后快照；返回 None 表示目标物品不存在或已删除。
    pub(crate) async fn update_item(
        &self,
        id: i64,
        input: UpdateStockItem,
        audit_user_id: Option<i64>,
    ) -> Result<Option<stock_item::Model>, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        let Some(item) = find_active_item_by_id_on_connection(&transaction, id).await? else {
            transaction.commit().await?;
            return Ok(None);
        };
        let previous = item.clone();
        let now = sqlite_now(&transaction).await?;
        let mut active_model: stock_item::ActiveModel = item.into();

        if let Some(name) = input.name {
            active_model.name = Set(name);
        }
        if let Some(sku) = input.sku {
            active_model.sku = Set(sku);
        }
        if let Some(category_id) = input.category_id {
            active_model.category_id = Set(category_id);
        }
        if let Some(attribute_template_id) = input.attribute_template_id {
            active_model.attribute_template_id = Set(attribute_template_id);
        }
        if let Some(image_file_id) = input.image_file_id {
            let owner_user_id = input
                .image_owner_user_id
                .ok_or_else(|| DbErr::Custom("item image owner missing".to_owned()))?;
            ensure_item_image_available(&transaction, image_file_id, owner_user_id).await?;
            active_model.image_file_id = Set(image_file_id);
        }
        if let Some(unit) = input.unit {
            active_model.unit = Set(unit);
        }
        if let Some(description) = input.description {
            active_model.description = Set(description);
        }
        if let Some(default_price) = input.default_price {
            active_model.default_price = Set(default_price);
        }
        if let Some(reorder_point) = input.reorder_point {
            active_model.reorder_point = Set(reorder_point);
        }
        active_model.updated_at = Set(now);

        let updated = active_model.update(&transaction).await?;
        if let Some(attributes) = input.attributes {
            replace_item_attributes_on_connection(&transaction, id, &attributes).await?;
        }
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "item",
                Some(updated.id),
                "updated",
                Some(item_updated_details(&previous, &updated)),
            )
            .await?;
        }
        transaction.commit().await?;

        Ok(Some(updated))
    }

    /// 软删除物品，并在同一事务内记录删除前快照；已有出入库记录可继续通过历史 ID 追溯。
    pub(crate) async fn soft_delete_item(
        &self,
        id: i64,
        audit_user_id: Option<i64>,
    ) -> Result<bool, DbErr>
    where
        C: TransactionTrait,
    {
        let transaction = self.database.begin().await?;
        let Some(item) = find_active_item_by_id_on_connection(&transaction, id).await? else {
            transaction.commit().await?;
            return Ok(false);
        };
        let previous = item.clone();
        let now = sqlite_now(&transaction).await?;
        let mut active_model: stock_item::ActiveModel = item.into();
        active_model.updated_at = Set(now.clone());
        active_model.deleted_at = Set(Some(now));
        let deleted = active_model.update(&transaction).await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "DELETE FROM stock_item_attributes WHERE item_id = ?",
                [id.into()],
            ))
            .await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "DELETE FROM stock_item_attribute_definitions WHERE owner_item_id = ?",
                [id.into()],
            ))
            .await?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "item",
                Some(deleted.id),
                "deleted",
                Some(item_deleted_details(&previous)),
            )
            .await?;
        }
        transaction.commit().await?;

        Ok(true)
    }

    async fn query_item_stock_summary(&self, item_id: i64) -> Result<(f64, f64), DbErr> {
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT
                    COALESCE(SUM(remaining_quantity), 0.0) AS current_quantity,
                    COALESCE(SUM(remaining_quantity * unit_cost), 0.0) AS inventory_value
                FROM stock_batches
                WHERE item_id = ?
                  AND remaining_quantity > 0
                "#,
                [item_id.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("stock item summary".to_owned()))?;

        Ok((
            row.try_get("", "current_quantity")?,
            row.try_get("", "inventory_value")?,
        ))
    }

    async fn query_item_batch_count(&self, item_id: i64) -> Result<u64, DbErr> {
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "SELECT COUNT(*) AS count FROM stock_batches WHERE item_id = ? AND remaining_quantity > 0",
                [item_id.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("stock item batch count".to_owned()))?;
        Ok(u64::try_from(row.try_get::<i64>("", "count")?).unwrap_or(0))
    }

    async fn query_item_catalog_counts(
        &self,
        base_sql: &str,
        values: Vec<Value>,
    ) -> Result<ItemCatalogCountsRecord, DbErr> {
        let sql = format!(
            r#"
            SELECT COUNT(*) AS total,
                   COALESCE(SUM(CASE WHEN stock_state IN ('out_of_stock', 'reorder_due') THEN 1 ELSE 0 END), 0) AS needs_attention,
                   COALESCE(SUM(CASE WHEN stock_state = 'out_of_stock' THEN 1 ELSE 0 END), 0) AS out_of_stock,
                   COALESCE(SUM(CASE WHEN stock_state = 'reorder_due' THEN 1 ELSE 0 END), 0) AS reorder_due,
                   COALESCE(SUM(CASE WHEN stock_state = 'needs_configuration' THEN 1 ELSE 0 END), 0) AS needs_configuration
            FROM ({base_sql}) catalog
            "#
        );
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                sql,
                values,
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("item catalog counts".to_owned()))?;
        Ok(ItemCatalogCountsRecord {
            total: sqlite_count(&row, "total")?,
            needs_attention: sqlite_count(&row, "needs_attention")?,
            out_of_stock: sqlite_count(&row, "out_of_stock")?,
            reorder_due: sqlite_count(&row, "reorder_due")?,
            needs_configuration: sqlite_count(&row, "needs_configuration")?,
        })
    }

    async fn query_catalog_attributes(
        &self,
        item_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<CatalogAttributeRecord>>, DbErr> {
        if item_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = vec!["?"; item_ids.len()].join(", ");
        let sql = format!(
            r#"
            SELECT attributes.item_id, definitions.field_name, attributes.value_json, attributes.unit
            FROM stock_item_attributes attributes
            JOIN stock_item_attribute_definitions definitions ON definitions.id = attributes.definition_id
            WHERE attributes.item_id IN ({placeholders})
              AND definitions.template_id IS NOT NULL
              AND definitions.catalog_visible = 1
            ORDER BY attributes.item_id, definitions.sort_order, definitions.id
            "#
        );
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                sql,
                item_ids.iter().copied().map(Into::into),
            ))
            .await?;
        let mut result = HashMap::new();
        for row in rows {
            let item_id: i64 = row.try_get("", "item_id")?;
            result
                .entry(item_id)
                .or_insert_with(Vec::new)
                .push(CatalogAttributeRecord {
                    name: row.try_get("", "field_name")?,
                    value_json: row.try_get("", "value_json")?,
                    unit: row.try_get("", "unit")?,
                });
        }
        Ok(result)
    }

    async fn query_item_stock_locations(
        &self,
        item_id: i64,
    ) -> Result<Vec<StockItemLocationRecord>, DbErr> {
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT
                    locations.id AS location_id,
                    locations.name AS location_name,
                    COALESCE(SUM(batches.remaining_quantity), 0.0) AS quantity,
                    COALESCE(SUM(batches.remaining_quantity * batches.unit_cost), 0.0) AS value,
                    COUNT(*) AS batch_count
                FROM stock_batches batches
                JOIN stock_locations locations ON locations.id = batches.location_id
                WHERE batches.item_id = ?
                  AND batches.remaining_quantity > 0
                GROUP BY locations.id, locations.name
                ORDER BY locations.name ASC, locations.id ASC
                "#,
                [item_id.into()],
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
                Ok(StockItemLocationRecord {
                    location_id: row.try_get("", "location_id")?,
                    location_name: row.try_get("", "location_name")?,
                    quantity: row.try_get("", "quantity")?,
                    value: row.try_get("", "value")?,
                    batch_count: row.try_get("", "batch_count")?,
                })
            })
            .collect()
    }
}

async fn find_active_item_by_id_on_connection<C>(
    connection: &C,
    id: i64,
) -> Result<Option<stock_item::Model>, DbErr>
where
    C: ConnectionTrait,
{
    stock_item::Entity::find_by_id(id)
        .filter(stock_item::Column::DeletedAt.is_null())
        .one(connection)
        .await
}

fn item_created_details(item: &stock_item::Model) -> String {
    json!({
        "name": item.name,
        "sku": item.sku,
        "category_id": item.category_id,
        "attribute_template_id": item.attribute_template_id,
        "image_file_id": item.image_file_id,
        "unit": item.unit,
        "default_price": item.default_price,
        "reorder_point": item.reorder_point
    })
    .to_string()
}

fn item_updated_details(previous: &stock_item::Model, updated: &stock_item::Model) -> String {
    json!({
        "changed_fields": item_changed_fields(previous, updated),
        "previous": item_audit_snapshot(previous),
        "new": item_audit_snapshot(updated)
    })
    .to_string()
}

fn item_deleted_details(item: &stock_item::Model) -> String {
    json!({
        "previous": item_audit_snapshot(item)
    })
    .to_string()
}

fn item_audit_snapshot(item: &stock_item::Model) -> serde_json::Value {
    json!({
        "name": item.name,
        "sku": item.sku,
        "category_id": item.category_id,
        "attribute_template_id": item.attribute_template_id,
        "image_file_id": item.image_file_id,
        "unit": item.unit,
        "description": item.description,
        "default_price": item.default_price,
        "reorder_point": item.reorder_point
    })
}

fn item_changed_fields(
    previous: &stock_item::Model,
    updated: &stock_item::Model,
) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if previous.name != updated.name {
        fields.push("name");
    }
    if previous.sku != updated.sku {
        fields.push("sku");
    }
    if previous.category_id != updated.category_id {
        fields.push("category_id");
    }
    if previous.attribute_template_id != updated.attribute_template_id {
        fields.push("attribute_template_id");
    }
    if previous.image_file_id != updated.image_file_id {
        fields.push("image_file_id");
    }
    if previous.unit != updated.unit {
        fields.push("unit");
    }
    if previous.description != updated.description {
        fields.push("description");
    }
    if previous.default_price != updated.default_price {
        fields.push("default_price");
    }
    if previous.reorder_point != updated.reorder_point {
        fields.push("reorder_point");
    }

    fields
}

pub(super) fn item_catalog_base_query(
    search_like: Option<&str>,
    category_id: Option<i64>,
    attribute_template_id: Option<i64>,
    field_filters: &[ItemCatalogFieldFilter],
) -> (String, Vec<Value>) {
    let mut sql = r#"
        SELECT stock_items.id, stock_items.name, stock_items.sku, stock_items.category_id,
               stock_items.attribute_template_id, stock_items.image_file_id, stock_items.unit,
               stock_items.description, stock_items.default_price, stock_items.reorder_point,
               stock_items.created_at, stock_items.updated_at, stock_items.deleted_at,
               categories.name AS category_name,
               COALESCE(inventory.current_quantity, 0.0) AS current_quantity,
               COALESCE(inventory.inventory_value, 0.0) AS inventory_value,
               COALESCE(inventory.location_count, 0) AS location_count,
               COALESCE(inventory.batch_count, 0) AS batch_count,
               CASE
                   WHEN COALESCE(inventory.current_quantity, 0.0) <= 0 THEN 'out_of_stock'
                   WHEN stock_items.reorder_point IS NOT NULL
                        AND inventory.current_quantity <= stock_items.reorder_point THEN 'reorder_due'
                   WHEN stock_items.reorder_point IS NULL THEN 'needs_configuration'
                   ELSE 'normal'
               END AS stock_state
        FROM stock_items
        LEFT JOIN stock_item_categories categories
               ON categories.id = stock_items.category_id AND categories.deleted_at IS NULL
        LEFT JOIN (
            SELECT item_id,
                   SUM(remaining_quantity) AS current_quantity,
                   SUM(remaining_quantity * unit_cost) AS inventory_value,
                   COUNT(DISTINCT location_id) AS location_count,
                   COUNT(*) AS batch_count
            FROM stock_batches
            WHERE remaining_quantity > 0
            GROUP BY item_id
        ) inventory ON inventory.item_id = stock_items.id
        WHERE stock_items.deleted_at IS NULL
        "#
    .to_owned();
    let mut values = Vec::new();
    if let Some(search_like) = search_like {
        search::append_item_search_filter(&mut sql, &mut values, search_like);
    }
    if let Some(category_id) = category_id {
        sql.push_str(" AND stock_items.category_id = ?");
        values.push(category_id.into());
    }
    if let Some(attribute_template_id) = attribute_template_id {
        sql.push_str(" AND stock_items.attribute_template_id = ?");
        values.push(attribute_template_id.into());
    }
    append_item_catalog_field_filters(&mut sql, &mut values, field_filters);
    (sql, values)
}

fn append_item_catalog_field_filters(
    sql: &mut String,
    values: &mut Vec<Value>,
    field_filters: &[ItemCatalogFieldFilter],
) {
    for filter in field_filters {
        match filter {
            ItemCatalogFieldFilter::Unit(filter_values) => {
                sql.push_str(" AND stock_items.unit IN (");
                append_bound_values(sql, values, filter_values);
                sql.push(')');
            }
            ItemCatalogFieldFilter::Location(filter_values) => {
                sql.push_str(
                    r#"
                    AND EXISTS (
                        SELECT 1
                        FROM stock_batches filter_batches
                        JOIN stock_locations filter_locations
                          ON filter_locations.id = filter_batches.location_id
                        WHERE filter_batches.item_id = stock_items.id
                          AND filter_batches.remaining_quantity > 0
                          AND filter_locations.name IN ("#,
                );
                append_bound_values(sql, values, filter_values);
                sql.push_str(") )");
            }
            ItemCatalogFieldFilter::Template {
                definition_id,
                values: filter_values,
            } => {
                sql.push_str(
                    r#"
                    AND EXISTS (
                        SELECT 1
                        FROM stock_item_attributes filter_attributes
                        JOIN stock_item_attribute_definitions filter_definitions
                          ON filter_definitions.id = filter_attributes.definition_id
                        WHERE filter_attributes.item_id = stock_items.id
                          AND filter_definitions.id = ?
                          AND filter_definitions.owner_item_id IS NULL
                          AND filter_definitions.searchable = 1
                          AND json_valid(filter_attributes.value_json)
                          AND CASE json_type(filter_attributes.value_json)
                              WHEN 'true' THEN 'true'
                              WHEN 'false' THEN 'false'
                              ELSE CAST(json_extract(filter_attributes.value_json, '$') AS TEXT)
                          END IN ("#,
                );
                values.push((*definition_id).into());
                append_bound_values(sql, values, filter_values);
                sql.push_str(") )");
            }
        }
    }
}

fn append_bound_values(sql: &mut String, values: &mut Vec<Value>, filter_values: &[String]) {
    for (index, value) in filter_values.iter().enumerate() {
        if index > 0 {
            sql.push_str(", ");
        }
        sql.push('?');
        values.push(value.clone().into());
    }
}

fn item_option_base_query(
    search_like: Option<&str>,
    category_id: Option<i64>,
    attribute_template_id: Option<i64>,
) -> (String, Vec<Value>) {
    let mut sql = r#"
        SELECT stock_items.id, stock_items.name, stock_items.sku, stock_items.category_id,
               stock_items.attribute_template_id,
               attribute_templates.default_inbound_template_id AS recommended_inbound_template_id,
               CASE WHEN recommended_inbound_templates.id IS NULL THEN 0 ELSE 1 END
                   AS recommended_inbound_template_available,
               categories.name AS category_name, stock_items.image_file_id, stock_items.unit
        FROM stock_items
        LEFT JOIN stock_item_categories categories
               ON categories.id = stock_items.category_id AND categories.deleted_at IS NULL
        LEFT JOIN stock_item_attribute_templates attribute_templates
               ON attribute_templates.id = stock_items.attribute_template_id
              AND attribute_templates.deleted_at IS NULL
        LEFT JOIN stock_inbound_templates recommended_inbound_templates
               ON recommended_inbound_templates.id = attribute_templates.default_inbound_template_id
              AND recommended_inbound_templates.deleted_at IS NULL
        WHERE stock_items.deleted_at IS NULL
        "#
    .to_owned();
    let mut values = Vec::new();
    if let Some(search_like) = search_like {
        search::append_item_search_filter(&mut sql, &mut values, search_like);
    }
    if let Some(category_id) = category_id {
        sql.push_str(" AND stock_items.category_id = ?");
        values.push(category_id.into());
    }
    if let Some(attribute_template_id) = attribute_template_id {
        sql.push_str(" AND stock_items.attribute_template_id = ?");
        values.push(attribute_template_id.into());
    }
    (sql, values)
}

pub(super) fn catalog_filter_sql(filter: CatalogStockFilter) -> &'static str {
    match filter {
        CatalogStockFilter::All => "",
        CatalogStockFilter::NeedsAttention => {
            "WHERE stock_state IN ('out_of_stock', 'reorder_due')"
        }
        CatalogStockFilter::OutOfStock => "WHERE stock_state = 'out_of_stock'",
        CatalogStockFilter::ReorderDue => "WHERE stock_state = 'reorder_due'",
        CatalogStockFilter::NeedsConfiguration => "WHERE stock_state = 'needs_configuration'",
    }
}

fn catalog_order_sql(sort: CatalogSort) -> &'static str {
    match sort {
        CatalogSort::ReplenishmentPriority => {
            "ORDER BY CASE stock_state WHEN 'out_of_stock' THEN 0 WHEN 'reorder_due' THEN 1 WHEN 'needs_configuration' THEN 2 ELSE 3 END, lower(name), lower(sku), id"
        }
        CatalogSort::Name => "ORDER BY lower(name), lower(sku), id",
        CatalogSort::QuantityAsc => "ORDER BY current_quantity ASC, id",
        CatalogSort::QuantityDesc => "ORDER BY current_quantity DESC, id",
        CatalogSort::InventoryValueDesc => "ORDER BY inventory_value DESC, id",
        CatalogSort::UpdatedDesc => "ORDER BY updated_at DESC, id",
    }
}

fn stock_item_from_query_row(row: &QueryResult) -> Result<stock_item::Model, DbErr> {
    Ok(stock_item::Model {
        id: row.try_get("", "id")?,
        name: row.try_get("", "name")?,
        sku: row.try_get("", "sku")?,
        category_id: row.try_get("", "category_id")?,
        attribute_template_id: row.try_get("", "attribute_template_id")?,
        image_file_id: row.try_get("", "image_file_id")?,
        unit: row.try_get("", "unit")?,
        description: row.try_get("", "description")?,
        default_price: row.try_get("", "default_price")?,
        reorder_point: row.try_get("", "reorder_point")?,
        created_at: row.try_get("", "created_at")?,
        updated_at: row.try_get("", "updated_at")?,
        deleted_at: row.try_get("", "deleted_at")?,
    })
}

fn stock_batch_from_query_row(row: QueryResult) -> Result<StockItemBatchRecord, DbErr> {
    Ok(StockItemBatchRecord {
        id: row.try_get("", "id")?,
        batch_no: row.try_get("", "batch_no")?,
        location_id: row.try_get("", "location_id")?,
        location_name: row.try_get("", "location_name")?,
        initial_quantity: row.try_get("", "initial_quantity")?,
        remaining_quantity: row.try_get("", "remaining_quantity")?,
        unit_cost: row.try_get("", "unit_cost")?,
        value: row.try_get("", "value")?,
        received_at: row.try_get("", "received_at")?,
        expires_at: row.try_get("", "expires_at")?,
    })
}

fn sqlite_count(row: &QueryResult, column: &str) -> Result<u64, DbErr> {
    Ok(u64::try_from(row.try_get::<i64>("", column)?).unwrap_or(0))
}

fn stock_state_code(current_quantity: f64, reorder_point: Option<f64>) -> &'static str {
    if current_quantity <= 0.0 {
        "out_of_stock"
    } else if reorder_point.is_some_and(|point| current_quantity <= point) {
        "reorder_due"
    } else if reorder_point.is_none() {
        "needs_configuration"
    } else {
        "normal"
    }
}

/// 整体替换物品属性；属性、文件绑定和物品基础资料共享调用方事务。
async fn replace_item_attributes_on_connection<C>(
    connection: &C,
    item_id: i64,
    attributes: &[ItemAttributeInput],
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    connection
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "DELETE FROM stock_item_attributes WHERE item_id = ?",
            [item_id.into()],
        ))
        .await?;
    let retained_private_ids = attributes
        .iter()
        .filter_map(|attribute| attribute.definition_id)
        .collect::<Vec<_>>();
    if retained_private_ids.is_empty() {
        connection
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "DELETE FROM stock_item_attribute_definitions WHERE owner_item_id = ?",
                [item_id.into()],
            ))
            .await?;
    } else {
        let placeholders = vec!["?"; retained_private_ids.len()].join(", ");
        let sql = format!(
            "DELETE FROM stock_item_attribute_definitions WHERE owner_item_id = ? AND id NOT IN ({placeholders})"
        );
        let mut values = vec![item_id.into()];
        values.extend(retained_private_ids.iter().copied().map(Into::into));
        connection
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                sql,
                values,
            ))
            .await?;
    }
    for attribute in attributes {
        validate_repository_input(attribute)?;
        let now = sqlite_now(connection).await?;
        let definition_id = if let Some(definition_id) = attribute.definition_id {
            let definition = item_attribute_definition::Entity::find_by_id(definition_id)
                .one(connection)
                .await?
                .ok_or_else(|| DbErr::Custom("item attribute definition missing".to_owned()))?;
            if definition.template_id.is_none() && definition.owner_item_id != Some(item_id) {
                return Err(DbErr::Custom(
                    "item attribute definition ownership mismatch".to_owned(),
                ));
            }
            if definition.owner_item_id == Some(item_id) {
                let mut active: item_attribute_definition::ActiveModel = definition.into();
                active.field_name = Set(attribute.field_name.clone());
                active.field_type = Set(attribute.field_type.clone());
                active.options_json = Set(attribute.options_json.clone());
                active.unit_mode = Set(attribute.unit_mode.clone());
                active.fixed_unit = Set(attribute.fixed_unit.clone());
                active.unit_options_json = Set(attribute.unit_options_json.clone());
                active.sort_order = Set(attribute.sort_order);
                active.updated_at = Set(now.clone());
                active.update(connection).await?;
            }
            definition_id
        } else {
            item_attribute_definition::Entity::insert(item_attribute_definition::ActiveModel {
                template_id: Set(None),
                owner_item_id: Set(Some(item_id)),
                field_name: Set(attribute.field_name.clone()),
                field_type: Set(attribute.field_type.clone()),
                required: Set(1),
                searchable: Set(0),
                catalog_visible: Set(0),
                options_json: Set(attribute.options_json.clone()),
                default_value: Set(None),
                unit_mode: Set(attribute.unit_mode.clone()),
                fixed_unit: Set(attribute.fixed_unit.clone()),
                unit_options_json: Set(attribute.unit_options_json.clone()),
                sort_order: Set(attribute.sort_order),
                created_at: Set(now.clone()),
                updated_at: Set(now.clone()),
                ..Default::default()
            })
            .exec(connection)
            .await?
            .last_insert_id
        };
        let result = item_attribute::Entity::insert(item_attribute::ActiveModel {
            item_id: Set(item_id),
            definition_id: Set(definition_id),
            value_json: Set(attribute.value_json.clone()),
            unit: Set(attribute.unit.clone()),
            sort_order: Set(attribute.sort_order),
            created_at: Set(now.clone()),
            updated_at: Set(now.clone()),
            ..Default::default()
        })
        .exec(connection)
        .await?;
        let Some(file_id) = attribute.file_object_id else {
            continue;
        };
        let Some(owner_id) = attribute.file_owner_user_id else {
            return Err(DbErr::Custom("item file owner missing".to_owned()));
        };
        let binding = connection.execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO storage_item_file_bindings (file_object_id, item_attribute_id, created_at)
            SELECT f.id, ?, ? FROM storage_file_objects f
            WHERE f.id = ? AND f.owner_user_id = ?
              AND f.mime_type IN ('image/png', 'image/jpeg', 'image/webp')
              AND NOT EXISTS (SELECT 1 FROM storage_item_file_bindings b WHERE b.file_object_id = f.id)
              AND NOT EXISTS (SELECT 1 FROM storage_inbound_file_bindings b WHERE b.file_object_id = f.id)
              AND NOT EXISTS (SELECT 1 FROM stock_items item WHERE item.image_file_id = f.id)
            "#,
            vec![result.last_insert_id.into(), now.into(), file_id.into(), owner_id.into()],
        )).await?;
        if binding.rows_affected() != 1 {
            return Err(DbErr::Custom("item file unavailable".to_owned()));
        }
    }
    Ok(())
}

/// 在调用方事务内确认主图属于操作者、类型正确且未被其它业务记录占用。
async fn ensure_item_image_available<C>(
    connection: &C,
    file_id: i64,
    owner_user_id: i64,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let row = connection
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT COUNT(*) AS count
            FROM storage_file_objects f
            WHERE f.id = ?
              AND f.owner_user_id = ?
              AND f.mime_type IN ('image/png', 'image/jpeg', 'image/webp')
              AND NOT EXISTS (SELECT 1 FROM stock_items item WHERE item.image_file_id = f.id)
              AND NOT EXISTS (SELECT 1 FROM storage_item_file_bindings b WHERE b.file_object_id = f.id)
              AND NOT EXISTS (SELECT 1 FROM storage_inbound_file_bindings b WHERE b.file_object_id = f.id)
            "#,
            vec![file_id.into(), owner_user_id.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("item image availability".to_owned()))?;
    let count: i64 = row.try_get("", "count")?;
    if count != 1 {
        return Err(DbErr::Custom(format!("item image unavailable:{file_id}")));
    }
    Ok(())
}

async fn list_item_attributes_on_connection<C>(
    connection: &C,
    item_id: i64,
) -> Result<Vec<super::ItemAttributeRecord>, DbErr>
where
    C: ConnectionTrait,
{
    let rows = connection.query_all(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        r#"
        SELECT a.id, a.item_id, a.definition_id, a.value_json, a.unit, a.sort_order,
               a.created_at, a.updated_at, d.template_id, d.owner_item_id, d.field_name,
               d.field_type, d.required, d.searchable, d.catalog_visible, d.options_json, d.default_value,
               d.unit_mode, d.fixed_unit, d.unit_options_json, d.sort_order AS definition_sort_order,
               d.created_at AS definition_created_at, d.updated_at AS definition_updated_at
        FROM stock_item_attributes a
        JOIN stock_item_attribute_definitions d ON d.id = a.definition_id
        WHERE a.item_id = ?
        ORDER BY a.sort_order, a.id
        "#,
        [item_id.into()],
    )).await?;
    rows.into_iter()
        .map(|row| {
            Ok(super::ItemAttributeRecord {
                attribute: item_attribute::Model {
                    id: row.try_get("", "id")?,
                    item_id: row.try_get("", "item_id")?,
                    definition_id: row.try_get("", "definition_id")?,
                    value_json: row.try_get("", "value_json")?,
                    unit: row.try_get("", "unit")?,
                    sort_order: row.try_get("", "sort_order")?,
                    created_at: row.try_get("", "created_at")?,
                    updated_at: row.try_get("", "updated_at")?,
                },
                definition: item_attribute_definition::Model {
                    id: row.try_get("", "definition_id")?,
                    template_id: row.try_get("", "template_id")?,
                    owner_item_id: row.try_get("", "owner_item_id")?,
                    field_name: row.try_get("", "field_name")?,
                    field_type: row.try_get("", "field_type")?,
                    required: row.try_get("", "required")?,
                    searchable: row.try_get("", "searchable")?,
                    catalog_visible: row.try_get("", "catalog_visible")?,
                    options_json: row.try_get("", "options_json")?,
                    default_value: row.try_get("", "default_value")?,
                    unit_mode: row.try_get("", "unit_mode")?,
                    fixed_unit: row.try_get("", "fixed_unit")?,
                    unit_options_json: row.try_get("", "unit_options_json")?,
                    sort_order: row.try_get("", "definition_sort_order")?,
                    created_at: row.try_get("", "definition_created_at")?,
                    updated_at: row.try_get("", "definition_updated_at")?,
                },
            })
        })
        .collect()
}
