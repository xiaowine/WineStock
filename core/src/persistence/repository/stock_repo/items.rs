//! 库存物品仓储操作。
//!
//! 本模块属于 `core` 持久化层，封装 `stock_items` 的创建、查询、更新、软删除、物品详情库存快照和物品审计写入。
//! service 不应直接拼接库存物品表查询。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DbErr, EntityTrait,
    QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use serde_json::json;

use super::{
    common::insert_audit_event_on_connection, search, CreateStockItem, ItemAttributeInput,
    ListStockItems, Page, StockItemBatchRecord, StockItemDetail, StockItemListRecord,
    StockItemLocationRecord, StockRepository, UpdateStockItem,
};
use crate::persistence::{
    entity::{item_attribute, stock_item},
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

    /// 查询未软删除物品详情，并附带当前库存、库位分布和有效批次摘要。
    pub(crate) async fn find_active_item_detail_by_id(
        &self,
        id: i64,
    ) -> Result<Option<StockItemDetail>, DbErr> {
        let Some(item) = self.find_active_item_by_id(id).await? else {
            return Ok(None);
        };
        let (current_quantity, inventory_value) = self.query_item_stock_summary(id).await?;
        let locations = self.query_item_stock_locations(id).await?;
        let batches = self.query_item_stock_batches(id).await?;
        let attributes = list_item_attributes_on_connection(self.database, id).await?;

        Ok(Some(StockItemDetail {
            item,
            current_quantity,
            inventory_value,
            locations,
            batches,
            attributes,
        }))
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

    /// 分页查询未软删除物品，支持物品/模板/当前库存扩展属性搜索和模板筛选。
    pub(crate) async fn list_active_items(
        &self,
        input: ListStockItems,
    ) -> Result<Page<StockItemListRecord>, DbErr> {
        let limit = input.page_size as i64;
        let offset = ((input.page.saturating_sub(1)) * input.page_size) as i64;
        let search_like = input
            .search
            .as_ref()
            .map(|search| format!("%{}%", search.to_lowercase()));

        let total = self
            .count_active_items(search_like.as_deref(), input.category_id)
            .await?;
        let item_models = self
            .query_active_items(search_like.as_deref(), input.category_id, limit, offset)
            .await?;
        let mut items = Vec::with_capacity(item_models.len());
        for item in item_models {
            let attributes = list_item_attributes_on_connection(self.database, item.id).await?;
            items.push(StockItemListRecord { item, attributes });
        }

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

    async fn count_active_items(
        &self,
        search_like: Option<&str>,
        category_id: Option<i64>,
    ) -> Result<u64, DbErr> {
        let row = self
            .database
            .query_one(stock_item_query(
                "COUNT(*) AS count",
                search_like,
                category_id,
                None,
                None,
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("stock item count".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count as u64)
    }

    async fn query_active_items(
        &self,
        search_like: Option<&str>,
        category_id: Option<i64>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<stock_item::Model>, DbErr> {
        let rows = self
            .database
            .query_all(stock_item_query(
                "id, name, sku, category_id, attribute_template_id, image_file_id, unit, description, default_price, reorder_point, created_at, updated_at, deleted_at",
                search_like,
                category_id,
                Some(limit),
                Some(offset),
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
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
            })
            .collect()
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
                    locations.code AS location_code,
                    locations.name AS location_name,
                    COALESCE(SUM(batches.remaining_quantity), 0.0) AS quantity,
                    COALESCE(SUM(batches.remaining_quantity * batches.unit_cost), 0.0) AS value,
                    COUNT(*) AS batch_count
                FROM stock_batches batches
                JOIN stock_locations locations ON locations.id = batches.location_id
                WHERE batches.item_id = ?
                  AND batches.remaining_quantity > 0
                GROUP BY locations.id, locations.code, locations.name
                ORDER BY locations.code ASC, locations.id ASC
                "#,
                [item_id.into()],
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
                Ok(StockItemLocationRecord {
                    location_id: row.try_get("", "location_id")?,
                    location_code: row.try_get("", "location_code")?,
                    location_name: row.try_get("", "location_name")?,
                    quantity: row.try_get("", "quantity")?,
                    value: row.try_get("", "value")?,
                    batch_count: row.try_get("", "batch_count")?,
                })
            })
            .collect()
    }

    async fn query_item_stock_batches(
        &self,
        item_id: i64,
    ) -> Result<Vec<StockItemBatchRecord>, DbErr> {
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT
                    batches.id,
                    batches.batch_no,
                    batches.location_id,
                    locations.code AS location_code,
                    locations.name AS location_name,
                    batches.initial_quantity,
                    batches.remaining_quantity,
                    batches.unit_cost,
                    batches.remaining_quantity * batches.unit_cost AS value,
                    batches.received_at,
                    batches.expires_at
                FROM stock_batches batches
                JOIN stock_locations locations ON locations.id = batches.location_id
                WHERE batches.item_id = ?
                  AND batches.remaining_quantity > 0
                ORDER BY batches.expires_at IS NULL ASC, batches.expires_at ASC, batches.received_at ASC, batches.id ASC
                "#,
                [item_id.into()],
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
                Ok(StockItemBatchRecord {
                    id: row.try_get("", "id")?,
                    batch_no: row.try_get("", "batch_no")?,
                    location_id: row.try_get("", "location_id")?,
                    location_code: row.try_get("", "location_code")?,
                    location_name: row.try_get("", "location_name")?,
                    initial_quantity: row.try_get("", "initial_quantity")?,
                    remaining_quantity: row.try_get("", "remaining_quantity")?,
                    unit_cost: row.try_get("", "unit_cost")?,
                    value: row.try_get("", "value")?,
                    received_at: row.try_get("", "received_at")?,
                    expires_at: row.try_get("", "expires_at")?,
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

fn stock_item_query(
    select_clause: &str,
    search_like: Option<&str>,
    category_id: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Statement {
    let mut sql = format!("SELECT {select_clause} FROM stock_items WHERE deleted_at IS NULL");
    let mut values = Vec::new();

    if let Some(search_like) = search_like {
        search::append_item_search_filter(&mut sql, &mut values, search_like);
    }
    if let Some(category_id) = category_id {
        sql.push_str(" AND category_id = ?");
        values.push(category_id.into());
    }
    if limit.is_some() {
        sql.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");
        values.push(limit.expect("limit checked").into());
        values.push(offset.unwrap_or(0).into());
    }

    Statement::from_sql_and_values(DatabaseBackend::Sqlite, sql, values)
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
    for attribute in attributes {
        validate_repository_input(attribute)?;
        let now = sqlite_now(connection).await?;
        let result = item_attribute::Entity::insert(item_attribute::ActiveModel {
            item_id: Set(item_id),
            template_field_id: Set(attribute.template_field_id),
            field_name: Set(attribute.field_name.clone()),
            field_type: Set(attribute.field_type.clone()),
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
) -> Result<Vec<item_attribute::Model>, DbErr>
where
    C: ConnectionTrait,
{
    item_attribute::Entity::find()
        .filter(item_attribute::Column::ItemId.eq(item_id))
        .order_by_asc(item_attribute::Column::SortOrder)
        .order_by_asc(item_attribute::Column::Id)
        .all(connection)
        .await
}
