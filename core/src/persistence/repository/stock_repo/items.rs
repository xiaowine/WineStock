//! 库存物品仓储操作。
//!
//! 本模块属于 `core` 持久化层，封装 `stock_items` 的创建、查询、更新、软删除和物品详情库存快照。
//! service 不应直接拼接库存物品表查询。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DbErr, EntityTrait,
    QueryFilter, Set, Statement,
};

use super::{
    search, CreateStockItem, ListStockItems, Page, StockItemBatchRecord, StockItemDetail,
    StockItemLocationRecord, StockRepository, UpdateStockItem,
};
use crate::persistence::{
    entity::stock_item,
    repository::{time::sqlite_now, validation::validate_repository_input},
};

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建未删除库存物品，并使用数据库统一时间戳填充时间字段。
    pub(crate) async fn create_item(
        &self,
        input: CreateStockItem,
    ) -> Result<stock_item::Model, DbErr> {
        validate_repository_input(&input)?;
        let now = sqlite_now(self.database).await?;
        let active_model = stock_item::ActiveModel {
            name: Set(input.name),
            sku: Set(input.sku),
            category_id: Set(input.category_id),
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
            .exec(self.database)
            .await?;

        self.find_active_item_by_id(result.last_insert_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created stock item".to_owned()))
    }

    /// 查询未软删除物品详情；软删除记录不会返回给业务服务层。
    pub(crate) async fn find_active_item_by_id(
        &self,
        id: i64,
    ) -> Result<Option<stock_item::Model>, DbErr> {
        stock_item::Entity::find_by_id(id)
            .filter(stock_item::Column::DeletedAt.is_null())
            .one(self.database)
            .await
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

        Ok(Some(StockItemDetail {
            item,
            current_quantity,
            inventory_value,
            locations,
            batches,
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
    ) -> Result<Page<stock_item::Model>, DbErr> {
        let limit = input.page_size as i64;
        let offset = ((input.page.saturating_sub(1)) * input.page_size) as i64;
        let search_like = input
            .search
            .as_ref()
            .map(|search| format!("%{}%", search.to_lowercase()));

        let total = self
            .count_active_items(search_like.as_deref(), input.category_id)
            .await?;
        let items = self
            .query_active_items(search_like.as_deref(), input.category_id, limit, offset)
            .await?;

        Ok(Page { items, total })
    }

    /// 更新未软删除物品；返回 None 表示目标物品不存在或已删除。
    pub(crate) async fn update_item(
        &self,
        id: i64,
        input: UpdateStockItem,
    ) -> Result<Option<stock_item::Model>, DbErr> {
        validate_repository_input(&input)?;
        let Some(item) = self.find_active_item_by_id(id).await? else {
            return Ok(None);
        };
        let now = sqlite_now(self.database).await?;
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

        let updated = active_model.update(self.database).await?;
        Ok(Some(updated))
    }

    /// 软删除物品；已有出入库记录可继续通过历史 ID 追溯。
    pub(crate) async fn soft_delete_item(&self, id: i64) -> Result<bool, DbErr> {
        let Some(item) = self.find_active_item_by_id(id).await? else {
            return Ok(false);
        };
        let now = sqlite_now(self.database).await?;
        let mut active_model: stock_item::ActiveModel = item.into();
        active_model.updated_at = Set(now.clone());
        active_model.deleted_at = Set(Some(now));
        active_model.update(self.database).await?;

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
                "id, name, sku, category_id, unit, description, default_price, reorder_point, created_at, updated_at, deleted_at",
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
                    location,
                    COALESCE(SUM(remaining_quantity), 0.0) AS quantity,
                    COALESCE(SUM(remaining_quantity * unit_cost), 0.0) AS value,
                    COUNT(*) AS batch_count
                FROM stock_batches
                WHERE item_id = ?
                  AND remaining_quantity > 0
                GROUP BY location
                ORDER BY location IS NULL ASC, location ASC
                "#,
                [item_id.into()],
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
                Ok(StockItemLocationRecord {
                    location: row.try_get("", "location")?,
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
                    id,
                    batch_no,
                    location,
                    initial_quantity,
                    remaining_quantity,
                    unit_cost,
                    remaining_quantity * unit_cost AS value,
                    received_at,
                    expires_at
                FROM stock_batches
                WHERE item_id = ?
                  AND remaining_quantity > 0
                ORDER BY expires_at IS NULL ASC, expires_at ASC, received_at ASC, id ASC
                "#,
                [item_id.into()],
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
                Ok(StockItemBatchRecord {
                    id: row.try_get("", "id")?,
                    batch_no: row.try_get("", "batch_no")?,
                    location: row.try_get("", "location")?,
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
