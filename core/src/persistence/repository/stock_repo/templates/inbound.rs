//! 入库属性模板仓储实现。
//!
//! 本模块属于 core 持久化层，只管理描述单次收货状态的模板，不管理物品固有属性。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set, TransactionTrait,
};

use super::super::{
    CreateInboundTemplate, InboundTemplateDetail, StockRepository, UpdateInboundTemplate,
};
use super::common::{
    audit_template_change, find_active_inbound_template, inbound_field_inputs,
    insert_inbound_template, list_inbound_fields, replace_inbound_fields,
};
use crate::persistence::{
    entity::inbound_template,
    repository::{time::sqlite_now, validation::validate_repository_input},
};

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建入库模板、字段和审计事件；任一步失败都会回滚。
    pub(crate) async fn create_inbound_template(
        &self,
        input: CreateInboundTemplate,
        audit_user_id: Option<i64>,
    ) -> Result<InboundTemplateDetail, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        let template =
            insert_inbound_template(&transaction, &input.name, input.description.clone()).await?;
        replace_inbound_fields(&transaction, template.id, &input.fields).await?;
        audit_template_change(
            &transaction,
            audit_user_id,
            "inbound_template",
            template.id,
            "created",
            &template.name,
            input.fields.len(),
        )
        .await?;
        transaction.commit().await?;
        self.find_active_inbound_template_by_id(template.id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created inbound template".to_owned()))
    }

    /// 查询单个有效入库模板及字段。
    pub(crate) async fn find_active_inbound_template_by_id(
        &self,
        id: i64,
    ) -> Result<Option<InboundTemplateDetail>, DbErr> {
        find_active_inbound_template(self.database, id).await
    }

    /// 查询有效入库模板名称是否被其他记录占用。
    pub(crate) async fn active_inbound_template_name_exists_except(
        &self,
        name: &str,
        except_id: Option<i64>,
    ) -> Result<bool, DbErr> {
        let mut query = inbound_template::Entity::find()
            .filter(inbound_template::Column::DeletedAt.is_null())
            .filter(inbound_template::Column::Name.eq(name));
        if let Some(except_id) = except_id {
            query = query.filter(inbound_template::Column::Id.ne(except_id));
        }
        Ok(query.one(self.database).await?.is_some())
    }

    /// 查询任意状态的同名模板，启动补齐据此避免恢复用户删除的数据。
    pub(crate) async fn inbound_template_name_exists(&self, name: &str) -> Result<bool, DbErr> {
        Ok(inbound_template::Entity::find()
            .filter(inbound_template::Column::Name.eq(name))
            .one(self.database)
            .await?
            .is_some())
    }

    /// 查询全部有效入库模板及字段。
    pub(crate) async fn list_active_inbound_templates(
        &self,
    ) -> Result<Vec<InboundTemplateDetail>, DbErr> {
        let templates = inbound_template::Entity::find()
            .filter(inbound_template::Column::DeletedAt.is_null())
            .order_by_asc(inbound_template::Column::Name)
            .order_by_asc(inbound_template::Column::Id)
            .all(self.database)
            .await?;
        let mut result = Vec::with_capacity(templates.len());
        for template in templates {
            let fields = list_inbound_fields(self.database, template.id).await?;
            result.push(InboundTemplateDetail { template, fields });
        }
        Ok(result)
    }

    /// 更新入库模板；字段整体替换和审计写入处于同一事务。
    pub(crate) async fn update_inbound_template(
        &self,
        id: i64,
        input: UpdateInboundTemplate,
        audit_user_id: Option<i64>,
    ) -> Result<Option<InboundTemplateDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        let Some(template) = inbound_template::Entity::find_by_id(id)
            .filter(inbound_template::Column::DeletedAt.is_null())
            .one(&transaction)
            .await?
        else {
            transaction.commit().await?;
            return Ok(None);
        };
        let now = sqlite_now(&transaction).await?;
        let mut active: inbound_template::ActiveModel = template.into();
        if let Some(name) = input.name {
            active.name = Set(name);
        }
        if let Some(description) = input.description {
            active.description = Set(description);
        }
        active.updated_at = Set(now);
        let updated = active.update(&transaction).await?;
        if let Some(fields) = input.fields {
            replace_inbound_fields(&transaction, id, &fields).await?;
        }
        let count = list_inbound_fields(&transaction, id).await?.len();
        audit_template_change(
            &transaction,
            audit_user_id,
            "inbound_template",
            id,
            "updated",
            &updated.name,
            count,
        )
        .await?;
        transaction.commit().await?;
        self.find_active_inbound_template_by_id(id).await
    }

    /// 复制入库模板及字段。
    pub(crate) async fn copy_inbound_template(
        &self,
        id: i64,
        new_name: String,
        audit_user_id: Option<i64>,
    ) -> Result<Option<InboundTemplateDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        let Some(source) = self.find_active_inbound_template_by_id(id).await? else {
            return Ok(None);
        };
        Ok(Some(
            self.create_inbound_template(
                CreateInboundTemplate {
                    name: new_name,
                    description: source.template.description,
                    fields: inbound_field_inputs(&source.fields),
                },
                audit_user_id,
            )
            .await?,
        ))
    }

    /// 软删除入库模板；历史明细保留实际属性快照。
    pub(crate) async fn soft_delete_inbound_template(
        &self,
        id: i64,
        audit_user_id: Option<i64>,
    ) -> Result<bool, DbErr>
    where
        C: TransactionTrait,
    {
        let transaction = self.database.begin().await?;
        let Some(template) = inbound_template::Entity::find_by_id(id)
            .filter(inbound_template::Column::DeletedAt.is_null())
            .one(&transaction)
            .await?
        else {
            transaction.commit().await?;
            return Ok(false);
        };
        let now = sqlite_now(&transaction).await?;
        let mut active: inbound_template::ActiveModel = template.into();
        active.updated_at = Set(now.clone());
        active.deleted_at = Set(Some(now));
        let deleted = active.update(&transaction).await?;
        let count = list_inbound_fields(&transaction, id).await?.len();
        audit_template_change(
            &transaction,
            audit_user_id,
            "inbound_template",
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
