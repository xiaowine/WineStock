//! 物品分类仓储操作。
//!
//! 本模块属于 core 持久化层，只管理分类元数据；属性模板和物品属性由独立模块负责。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set, TransactionTrait,
};
use serde_json::json;

use super::{
    common::insert_audit_event_on_connection, CreateItemCategory, StockRepository,
    UpdateItemCategory,
};
use crate::persistence::{
    entity::stock_item_category,
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
        let result = stock_item_category::Entity::insert(stock_item_category::ActiveModel {
            name: Set(input.name),
            description: Set(input.description),
            sort_order: Set(input.sort_order),
            created_at: Set(now.clone()),
            updated_at: Set(now),
            deleted_at: Set(None),
            ..Default::default()
        })
        .exec(&transaction)
        .await?;
        let model = stock_item_category::Entity::find_by_id(result.last_insert_id)
            .one(&transaction)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created item category".to_owned()))?;
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
    ) -> Result<bool, DbErr>
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
            return Ok(false);
        };
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
        Ok(true)
    }
}
