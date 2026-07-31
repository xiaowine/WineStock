//! 物品分类仓储操作。
//!
//! 本模块属于 core 持久化层，只管理分类元数据；属性模板和物品属性由独立模块负责。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, Statement, TransactionSession, TransactionTrait,
};
use serde_json::json;
use std::collections::HashMap;

use super::{
    common::insert_audit_event_on_connection, CreateItemCategory, StockRepository,
    UpdateItemCategory,
};
use crate::persistence::{
    entity::{stock_item, stock_item_category},
    repository::{time::sqlite_now, validation::validate_repository_input},
};

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建分类并在同一事务中写入审计事件。
    pub(crate) async fn create_item_category(
        &self,
        input: CreateItemCategory,
        audit_user_id: Option<i64>,
    ) -> Result<stock_item_category::Model, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        let model = stock_item_category::Entity::insert(stock_item_category::ActiveModel {
            name: Set(input.name),
            description: Set(input.description),
            sort_order: Set(input.sort_order),
            created_at: Set(now.clone()),
            updated_at: Set(now),
            deleted_at: Set(None),
            ..Default::default()
        })
        .exec_with_returning(&transaction)
        .await?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "item_category",
                Some(model.id),
                "created",
                Some(json!({"name": model.name, "sort_order": model.sort_order}).to_string()),
            )
            .await?;
        }
        transaction.commit().await?;
        Ok(model)
    }

    /// 查询全部有效分类并按展示顺序返回。
    pub(crate) async fn list_active_item_categories(
        &self,
    ) -> Result<Vec<stock_item_category::Model>, DbErr> {
        stock_item_category::Entity::find()
            .filter(stock_item_category::Column::DeletedAt.is_null())
            .order_by_asc(stock_item_category::Column::SortOrder)
            .order_by_asc(stock_item_category::Column::Id)
            .all(self.database)
            .await
    }

    /// 查询单个有效分类。
    pub(crate) async fn find_active_item_category_by_id(
        &self,
        id: i64,
    ) -> Result<Option<stock_item_category::Model>, DbErr> {
        stock_item_category::Entity::find_by_id(id)
            .filter(stock_item_category::Column::DeletedAt.is_null())
            .one(self.database)
            .await
    }

    /// 统计当前有效物品对指定分类的直接引用数。
    pub(crate) async fn active_item_category_usage_count(&self, id: i64) -> Result<u64, DbErr> {
        stock_item::Entity::find()
            .filter(stock_item::Column::DeletedAt.is_null())
            .filter(stock_item::Column::CategoryId.eq(id))
            .count(self.database)
            .await
    }

    /// 批量统计有效物品按分类的直接引用数，供分类列表避免逐行查询。
    pub(crate) async fn active_item_category_usage_counts(
        &self,
    ) -> Result<HashMap<i64, u64>, DbErr> {
        self.database
            .query_all_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT category_id, COUNT(*) AS item_usage_count FROM stock_items WHERE deleted_at IS NULL AND category_id IS NOT NULL GROUP BY category_id".to_owned(),
            ))
            .await?
            .into_iter()
            .map(|row| {
                Ok((
                    row.try_get("", "category_id")?,
                    row.try_get::<i64>("", "item_usage_count")? as u64,
                ))
            })
            .collect()
    }

    /// 查询有效分类名称是否被其他记录占用。
    pub(crate) async fn active_item_category_name_exists_except(
        &self,
        name: &str,
        except_id: Option<i64>,
    ) -> Result<bool, DbErr> {
        let mut query = stock_item_category::Entity::find()
            .filter(stock_item_category::Column::DeletedAt.is_null())
            .filter(stock_item_category::Column::Name.eq(name));
        if let Some(except_id) = except_id {
            query = query.filter(stock_item_category::Column::Id.ne(except_id));
        }
        Ok(query.one(self.database).await?.is_some())
    }

    /// 查询任意状态的同名分类，启动补齐不会恢复软删除记录。
    pub(crate) async fn item_category_name_exists(&self, name: &str) -> Result<bool, DbErr> {
        Ok(stock_item_category::Entity::find()
            .filter(stock_item_category::Column::Name.eq(name))
            .one(self.database)
            .await?
            .is_some())
    }

    /// 更新分类并记录审计摘要。
    pub(crate) async fn update_item_category(
        &self,
        id: i64,
        input: UpdateItemCategory,
        audit_user_id: Option<i64>,
    ) -> Result<Option<stock_item_category::Model>, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        let Some(model) = stock_item_category::Entity::find_by_id(id)
            .filter(stock_item_category::Column::DeletedAt.is_null())
            .one(&transaction)
            .await?
        else {
            transaction.commit().await?;
            return Ok(None);
        };
        let now = sqlite_now(&transaction).await?;
        let mut active: stock_item_category::ActiveModel = model.into();
        if let Some(name) = input.name {
            active.name = Set(name);
        }
        if let Some(description) = input.description {
            active.description = Set(description);
        }
        if let Some(sort_order) = input.sort_order {
            active.sort_order = Set(sort_order);
        }
        active.updated_at = Set(now);
        let updated = active.update(&transaction).await?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "item_category",
                Some(id),
                "updated",
                Some(json!({"name": updated.name, "sort_order": updated.sort_order}).to_string()),
            )
            .await?;
        }
        transaction.commit().await?;
        Ok(Some(updated))
    }

    /// 软删除分类；物品基础资料和历史单据保持不变。
    pub(crate) async fn soft_delete_item_category(
        &self,
        id: i64,
        audit_user_id: Option<i64>,
    ) -> Result<Option<u64>, DbErr>
    where
        C: TransactionTrait,
    {
        let transaction = self.database.begin().await?;
        let Some(model) = stock_item_category::Entity::find_by_id(id)
            .filter(stock_item_category::Column::DeletedAt.is_null())
            .one(&transaction)
            .await?
        else {
            transaction.commit().await?;
            return Ok(None);
        };
        let affected_active_item_count = stock_item::Entity::find()
            .filter(stock_item::Column::DeletedAt.is_null())
            .filter(stock_item::Column::CategoryId.eq(id))
            .count(&transaction)
            .await?;
        let now = sqlite_now(&transaction).await?;
        let mut active: stock_item_category::ActiveModel = model.into();
        active.updated_at = Set(now.clone());
        active.deleted_at = Set(Some(now));
        let deleted = active.update(&transaction).await?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "item_category",
                Some(id),
                "deleted",
                Some(json!({"name": deleted.name}).to_string()),
            )
            .await?;
        }
        transaction.commit().await?;
        Ok(Some(affected_active_item_count))
    }
}
