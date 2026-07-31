//! 物品属性模板仓储实现。
//!
//! 本模块属于 core 持久化层，管理可选录入预设；物品实际属性仍由物品仓储独立保存。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, Statement, TransactionSession, TransactionTrait,
};
use std::collections::HashMap;

use super::super::{
    CreateItemAttributeTemplate, ItemAttributeTemplateDetail, StockRepository,
    UpdateItemAttributeTemplate,
};
use super::common::{
    audit_template_change, find_active_item_attribute_template, insert_item_attribute_template,
    item_attribute_field_inputs, list_item_attribute_fields, replace_item_attribute_fields,
};
use crate::persistence::{
    entity::{item_attribute_template, stock_item},
    repository::{time::sqlite_now, validation::validate_repository_input},
};

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建物品属性模板、预设字段和审计事件。
    pub(crate) async fn create_item_attribute_template(
        &self,
        input: CreateItemAttributeTemplate,
        audit_user_id: Option<i64>,
    ) -> Result<ItemAttributeTemplateDetail, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        let template =
            insert_item_attribute_template(&transaction, &input.name, input.description.clone())
                .await?;
        replace_item_attribute_fields(&transaction, template.id, &input.fields).await?;
        audit_template_change(
            &transaction,
            audit_user_id,
            "item_attribute_template",
            template.id,
            "created",
            &template.name,
            input.fields.len(),
        )
        .await?;
        transaction.commit().await?;
        self.find_active_item_attribute_template_by_id(template.id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created item attribute template".to_owned()))
    }

    /// 查询单个有效物品属性模板及预设字段。
    pub(crate) async fn find_active_item_attribute_template_by_id(
        &self,
        id: i64,
    ) -> Result<Option<ItemAttributeTemplateDetail>, DbErr> {
        find_active_item_attribute_template(self.database, id).await
    }

    /// 统计当前有效物品对指定属性模板的直接引用数。
    pub(crate) async fn active_item_attribute_template_usage_count(
        &self,
        id: i64,
    ) -> Result<u64, DbErr> {
        stock_item::Entity::find()
            .filter(stock_item::Column::DeletedAt.is_null())
            .filter(stock_item::Column::AttributeTemplateId.eq(id))
            .count(self.database)
            .await
    }

    /// 批量统计有效物品按属性模板的直接引用数，供模板列表避免逐行查询。
    pub(crate) async fn active_item_attribute_template_usage_counts(
        &self,
    ) -> Result<HashMap<i64, u64>, DbErr> {
        self.database
            .query_all_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT attribute_template_id, COUNT(*) AS item_usage_count FROM stock_items WHERE deleted_at IS NULL AND attribute_template_id IS NOT NULL GROUP BY attribute_template_id".to_owned(),
            ))
            .await?
            .into_iter()
            .map(|row| {
                Ok((
                    row.try_get("", "attribute_template_id")?,
                    row.try_get::<i64>("", "item_usage_count")? as u64,
                ))
            })
            .collect()
    }

    /// 查询有效物品属性模板名称是否被其他记录占用。
    pub(crate) async fn active_item_attribute_template_name_exists_except(
        &self,
        name: &str,
        except_id: Option<i64>,
    ) -> Result<bool, DbErr> {
        let mut query = item_attribute_template::Entity::find()
            .filter(item_attribute_template::Column::DeletedAt.is_null())
            .filter(item_attribute_template::Column::Name.eq(name));
        if let Some(except_id) = except_id {
            query = query.filter(item_attribute_template::Column::Id.ne(except_id));
        }
        Ok(query.one(self.database).await?.is_some())
    }

    /// 查询任意状态的同名模板，启动补齐不会恢复软删除数据。
    pub(crate) async fn item_attribute_template_name_exists(
        &self,
        name: &str,
    ) -> Result<bool, DbErr> {
        Ok(item_attribute_template::Entity::find()
            .filter(item_attribute_template::Column::Name.eq(name))
            .one(self.database)
            .await?
            .is_some())
    }

    /// 查询全部有效物品属性模板及预设字段，并保持创建顺序。
    pub(crate) async fn list_active_item_attribute_templates(
        &self,
    ) -> Result<Vec<ItemAttributeTemplateDetail>, DbErr> {
        let templates = item_attribute_template::Entity::find()
            .filter(item_attribute_template::Column::DeletedAt.is_null())
            .order_by_asc(item_attribute_template::Column::Id)
            .all(self.database)
            .await?;
        let mut result = Vec::with_capacity(templates.len());
        for template in templates {
            let fields = list_item_attribute_fields(self.database, template.id).await?;
            result.push(ItemAttributeTemplateDetail { template, fields });
        }
        Ok(result)
    }

    /// 更新物品属性模板。
    pub(crate) async fn update_item_attribute_template(
        &self,
        id: i64,
        input: UpdateItemAttributeTemplate,
        audit_user_id: Option<i64>,
    ) -> Result<Option<ItemAttributeTemplateDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        let Some(template) = item_attribute_template::Entity::find_by_id(id)
            .filter(item_attribute_template::Column::DeletedAt.is_null())
            .one(&transaction)
            .await?
        else {
            transaction.commit().await?;
            return Ok(None);
        };
        let now = sqlite_now(&transaction).await?;
        // 全站至多一个默认：置真前在同一事务里清除其它有效模板的默认标记。
        if input.is_default == Some(true) && !template.is_default {
            transaction
                .execute_raw(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    "UPDATE stock_item_attribute_templates SET is_default = 0, updated_at = ? WHERE is_default = 1",
                    vec![now.clone().into()],
                ))
                .await?;
        }
        let mut active: item_attribute_template::ActiveModel = template.into();
        if let Some(name) = input.name {
            active.name = Set(name);
        }
        if let Some(description) = input.description {
            active.description = Set(description);
        }
        if let Some(is_default) = input.is_default {
            active.is_default = Set(is_default);
        }
        active.updated_at = Set(now);
        let updated = active.update(&transaction).await?;
        if let Some(fields) = input.fields {
            replace_item_attribute_fields(&transaction, id, &fields).await?;
        }
        let count = list_item_attribute_fields(&transaction, id).await?.len();
        audit_template_change(
            &transaction,
            audit_user_id,
            "item_attribute_template",
            id,
            "updated",
            &updated.name,
            count,
        )
        .await?;
        transaction.commit().await?;
        self.find_active_item_attribute_template_by_id(id).await
    }

    /// 复制物品属性模板。
    pub(crate) async fn copy_item_attribute_template(
        &self,
        id: i64,
        new_name: String,
        audit_user_id: Option<i64>,
    ) -> Result<Option<ItemAttributeTemplateDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        let Some(source) = self.find_active_item_attribute_template_by_id(id).await? else {
            return Ok(None);
        };
        Ok(Some(
            self.create_item_attribute_template(
                CreateItemAttributeTemplate {
                    name: new_name,
                    description: source.template.description,
                    fields: item_attribute_field_inputs(&source.fields),
                },
                audit_user_id,
            )
            .await?,
        ))
    }

    /// 删除模板时清空物品引用，并删除模板定义及其属性值。
    pub(crate) async fn soft_delete_item_attribute_template(
        &self,
        id: i64,
        audit_user_id: Option<i64>,
    ) -> Result<Option<u64>, DbErr>
    where
        C: TransactionTrait,
    {
        let transaction = self.database.begin().await?;
        let Some(template) = item_attribute_template::Entity::find_by_id(id)
            .filter(item_attribute_template::Column::DeletedAt.is_null())
            .one(&transaction)
            .await?
        else {
            transaction.commit().await?;
            return Ok(None);
        };
        let now = sqlite_now(&transaction).await?;
        let affected_active_item_count = stock_item::Entity::find()
            .filter(stock_item::Column::DeletedAt.is_null())
            .filter(stock_item::Column::AttributeTemplateId.eq(id))
            .count(&transaction)
            .await?;
        let count = list_item_attribute_fields(&transaction, id).await?.len();
        transaction.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "UPDATE stock_items SET attribute_template_id = NULL, updated_at = ? WHERE attribute_template_id = ?",
            vec![now.clone().into(), id.into()],
        )).await?;
        transaction
            .execute_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "DELETE FROM stock_item_attribute_definitions WHERE template_id = ?",
                [id.into()],
            ))
            .await?;
        let mut active: item_attribute_template::ActiveModel = template.into();
        active.updated_at = Set(now.clone());
        active.deleted_at = Set(Some(now));
        let deleted = active.update(&transaction).await?;
        audit_template_change(
            &transaction,
            audit_user_id,
            "item_attribute_template",
            id,
            "deleted",
            &deleted.name,
            count,
        )
        .await?;
        transaction.commit().await?;
        Ok(Some(affected_active_item_count))
    }
}
