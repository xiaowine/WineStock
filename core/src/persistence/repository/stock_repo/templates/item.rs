//! 物品属性模板仓储实现。
//!
//! 本模块属于 core 持久化层，管理可选录入预设；物品实际属性仍由物品仓储独立保存。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DbErr, EntityTrait,
    QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};

use super::super::{
    CreateItemAttributeTemplate, ItemAttributeTemplateDetail, StockRepository,
    UpdateItemAttributeTemplate,
};
use super::common::{
    audit_template_change, find_active_item_attribute_template, insert_item_attribute_template,
    item_attribute_field_inputs, list_item_attribute_fields, replace_item_attribute_fields,
};
use crate::persistence::{
    entity::item_attribute_template,
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
        let template = insert_item_attribute_template(
            &transaction,
            &input.name,
            input.description.clone(),
            input.default_inbound_template_id,
        )
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

    /// 查询全部有效物品属性模板及预设字段。
    pub(crate) async fn list_active_item_attribute_templates(
        &self,
    ) -> Result<Vec<ItemAttributeTemplateDetail>, DbErr> {
        let templates = item_attribute_template::Entity::find()
            .filter(item_attribute_template::Column::DeletedAt.is_null())
            .order_by_asc(item_attribute_template::Column::Name)
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

    /// 更新物品属性模板及默认入库模板推荐。
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
        let mut active: item_attribute_template::ActiveModel = template.into();
        if let Some(name) = input.name {
            active.name = Set(name);
        }
        if let Some(description) = input.description {
            active.description = Set(description);
        }
        if let Some(default_id) = input.default_inbound_template_id {
            active.default_inbound_template_id = Set(default_id);
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

    /// 复制物品属性模板及其默认入库模板推荐。
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
                    default_inbound_template_id: source.template.default_inbound_template_id,
                    fields: item_attribute_field_inputs(&source.fields),
                },
                audit_user_id,
            )
            .await?,
        ))
    }

    /// 判断物品属性模板是否仍被有效物品引用。
    pub(crate) async fn active_items_reference_item_attribute_template(
        &self,
        template_id: i64,
    ) -> Result<bool, DbErr> {
        let row = self.database.query_one(Statement::from_sql_and_values(DatabaseBackend::Sqlite, "SELECT COUNT(*) AS count FROM stock_items WHERE attribute_template_id = ? AND deleted_at IS NULL", [template_id.into()])).await?.ok_or_else(|| DbErr::RecordNotFound("item attribute template reference count".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;
        Ok(count > 0)
    }

    /// 软删除未被有效物品引用的物品属性模板。
    pub(crate) async fn soft_delete_item_attribute_template(
        &self,
        id: i64,
        audit_user_id: Option<i64>,
    ) -> Result<bool, DbErr>
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
            return Ok(false);
        };
        let now = sqlite_now(&transaction).await?;
        let mut active: item_attribute_template::ActiveModel = template.into();
        active.updated_at = Set(now.clone());
        active.deleted_at = Set(Some(now));
        let deleted = active.update(&transaction).await?;
        let count = list_item_attribute_fields(&transaction, id).await?.len();
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
        Ok(true)
    }
}
