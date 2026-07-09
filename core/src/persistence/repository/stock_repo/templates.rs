//! 库存模板仓储操作。
//!
//! 本模块属于 `core` 持久化层，封装模板主表、字段定义和模板审计事件的事务写入与读取。
//! 它不处理模板字段的业务级组合校验。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DbErr, EntityTrait,
    QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use serde_json::json;

use super::{
    common::insert_audit_event_on_connection, CreateStockTemplate, StockRepository,
    StockTemplateDetail, TemplateFieldInput, UpdateStockTemplate,
};
use crate::persistence::{
    entity::{stock_template, stock_template_field},
    repository::{time::sqlite_now, validation::validate_repository_input},
};

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建模板和字段定义；父模板、字段和可选审计事件必须在同一事务内写入。
    pub(crate) async fn create_template(
        &self,
        input: CreateStockTemplate,
        audit_user_id: Option<i64>,
    ) -> Result<StockTemplateDetail, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        let template =
            insert_template_on_connection(&transaction, &input.name, input.description.clone())
                .await?;
        replace_template_fields_on_connection(&transaction, template.id, &input.fields).await?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "template",
                Some(template.id),
                "created",
                Some(template_created_details(&template, input.fields.len())),
            )
            .await?;
        }
        transaction.commit().await?;

        self.find_active_template_by_id(template.id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created stock template".to_owned()))
    }

    /// 查询未软删除模板详情。
    pub(crate) async fn find_active_template_by_id(
        &self,
        id: i64,
    ) -> Result<Option<StockTemplateDetail>, DbErr> {
        let Some(template) = stock_template::Entity::find_by_id(id)
            .filter(stock_template::Column::DeletedAt.is_null())
            .one(self.database)
            .await?
        else {
            return Ok(None);
        };
        let fields = list_template_fields_on_connection(self.database, id).await?;

        Ok(Some(StockTemplateDetail { template, fields }))
    }

    /// 查询指定模板名称是否已有其他未软删除模板占用。
    pub(crate) async fn active_template_name_exists_except(
        &self,
        name: &str,
        except_id: Option<i64>,
    ) -> Result<bool, DbErr> {
        let mut query = stock_template::Entity::find()
            .filter(stock_template::Column::DeletedAt.is_null())
            .filter(stock_template::Column::Name.eq(name));
        if let Some(except_id) = except_id {
            query = query.filter(stock_template::Column::Id.ne(except_id));
        }

        Ok(query.one(self.database).await?.is_some())
    }

    /// 查询指定模板名称是否已存在；启动补齐用它避免恢复用户软删除的默认模板。
    pub(crate) async fn template_name_exists(&self, name: &str) -> Result<bool, DbErr> {
        Ok(stock_template::Entity::find()
            .filter(stock_template::Column::Name.eq(name))
            .one(self.database)
            .await?
            .is_some())
    }

    /// 查询全部未软删除模板，字段按模板逐个加载以保持业务结构清晰。
    pub(crate) async fn list_active_templates(&self) -> Result<Vec<StockTemplateDetail>, DbErr> {
        let templates = stock_template::Entity::find()
            .filter(stock_template::Column::DeletedAt.is_null())
            .order_by_asc(stock_template::Column::Name)
            .order_by_asc(stock_template::Column::Id)
            .all(self.database)
            .await?;
        let mut result = Vec::with_capacity(templates.len());
        for template in templates {
            let fields = list_template_fields_on_connection(self.database, template.id).await?;
            result.push(StockTemplateDetail { template, fields });
        }

        Ok(result)
    }

    /// 更新模板和可选字段定义；字段替换、模板更新时间和可选审计事件必须在同一事务内完成。
    pub(crate) async fn update_template(
        &self,
        id: i64,
        input: UpdateStockTemplate,
        audit_user_id: Option<i64>,
    ) -> Result<Option<StockTemplateDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        let Some(template) = stock_template::Entity::find_by_id(id)
            .filter(stock_template::Column::DeletedAt.is_null())
            .one(&transaction)
            .await?
        else {
            transaction.commit().await?;
            return Ok(None);
        };
        let previous_fields = list_template_fields_on_connection(&transaction, id).await?;
        let previous = StockTemplateDetail {
            template: template.clone(),
            fields: previous_fields,
        };
        let fields_replaced = input.fields.is_some();
        let now = sqlite_now(&transaction).await?;
        let mut active_model: stock_template::ActiveModel = template.into();
        if let Some(name) = input.name {
            active_model.name = Set(name);
        }
        if let Some(description) = input.description {
            active_model.description = Set(description);
        }
        active_model.updated_at = Set(now);
        active_model.update(&transaction).await?;

        if let Some(fields) = input.fields {
            replace_template_fields_on_connection(&transaction, id, &fields).await?;
        }

        let updated = find_active_template_by_id_on_connection(&transaction, id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("updated stock template".to_owned()))?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "template",
                Some(id),
                "updated",
                Some(template_updated_details(
                    &previous,
                    &updated,
                    fields_replaced,
                )),
            )
            .await?;
        }
        transaction.commit().await?;
        Ok(Some(updated))
    }

    /// 复制模板及其字段定义，并在同一事务内记录新模板创建审计事件。
    pub(crate) async fn copy_template(
        &self,
        id: i64,
        new_name: String,
        audit_user_id: Option<i64>,
    ) -> Result<Option<StockTemplateDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        let Some(source) = self.find_active_template_by_id(id).await? else {
            return Ok(None);
        };
        let fields: Vec<TemplateFieldInput> = source
            .fields
            .iter()
            .map(|field| TemplateFieldInput {
                field_name: field.field_name.clone(),
                field_type: field.field_type.clone(),
                required: field.required != 0,
                searchable: field.searchable != 0,
                options_json: field.options_json.clone(),
                default_value: field.default_value.clone(),
                sort_order: field.sort_order,
            })
            .collect();

        let transaction = self.database.begin().await?;
        let template = insert_template_on_connection(
            &transaction,
            &new_name,
            source.template.description.clone(),
        )
        .await?;
        replace_template_fields_on_connection(&transaction, template.id, &fields).await?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "template",
                Some(template.id),
                "created",
                Some(template_copied_details(&template, id, fields.len())),
            )
            .await?;
        }
        transaction.commit().await?;

        self.find_active_template_by_id(template.id).await
    }

    /// 判断模板是否仍被未删除物品引用。
    pub(crate) async fn active_items_reference_template(
        &self,
        template_id: i64,
    ) -> Result<bool, DbErr> {
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT COUNT(*) AS count
                FROM stock_items
                WHERE category_id = ? AND deleted_at IS NULL
                "#,
                [template_id.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("stock template item count".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count > 0)
    }

    /// 软删除模板并记录删除前快照；调用方必须先确认未被有效物品引用。
    pub(crate) async fn soft_delete_template(
        &self,
        id: i64,
        audit_user_id: Option<i64>,
    ) -> Result<bool, DbErr>
    where
        C: TransactionTrait,
    {
        let transaction = self.database.begin().await?;
        let Some(template) = stock_template::Entity::find_by_id(id)
            .filter(stock_template::Column::DeletedAt.is_null())
            .one(&transaction)
            .await?
        else {
            transaction.commit().await?;
            return Ok(false);
        };
        let fields = list_template_fields_on_connection(&transaction, id).await?;
        let previous = StockTemplateDetail {
            template: template.clone(),
            fields,
        };
        let now = sqlite_now(&transaction).await?;
        let mut active_model: stock_template::ActiveModel = template.into();
        active_model.updated_at = Set(now.clone());
        active_model.deleted_at = Set(Some(now));
        let deleted = active_model.update(&transaction).await?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "template",
                Some(deleted.id),
                "deleted",
                Some(template_deleted_details(&previous)),
            )
            .await?;
        }
        transaction.commit().await?;

        Ok(true)
    }
}

async fn find_active_template_by_id_on_connection<C>(
    connection: &C,
    id: i64,
) -> Result<Option<StockTemplateDetail>, DbErr>
where
    C: ConnectionTrait,
{
    let Some(template) = stock_template::Entity::find_by_id(id)
        .filter(stock_template::Column::DeletedAt.is_null())
        .one(connection)
        .await?
    else {
        return Ok(None);
    };
    let fields = list_template_fields_on_connection(connection, id).await?;

    Ok(Some(StockTemplateDetail { template, fields }))
}

fn template_created_details(template: &stock_template::Model, field_count: usize) -> String {
    json!({
        "name": template.name,
        "description": template.description,
        "field_count": field_count
    })
    .to_string()
}

fn template_copied_details(
    template: &stock_template::Model,
    source_template_id: i64,
    field_count: usize,
) -> String {
    json!({
        "name": template.name,
        "description": template.description,
        "field_count": field_count,
        "source_template_id": source_template_id
    })
    .to_string()
}

fn template_updated_details(
    previous: &StockTemplateDetail,
    updated: &StockTemplateDetail,
    fields_replaced: bool,
) -> String {
    json!({
        "changed_fields": template_changed_fields(previous, updated, fields_replaced),
        "fields_replaced": fields_replaced,
        "previous": template_audit_snapshot(previous),
        "new": template_audit_snapshot(updated)
    })
    .to_string()
}

fn template_deleted_details(template: &StockTemplateDetail) -> String {
    json!({
        "previous": template_audit_snapshot(template)
    })
    .to_string()
}

fn template_audit_snapshot(template: &StockTemplateDetail) -> serde_json::Value {
    json!({
        "name": template.template.name,
        "description": template.template.description,
        "field_count": template.fields.len(),
        "field_names": template
            .fields
            .iter()
            .map(|field| field.field_name.as_str())
            .collect::<Vec<_>>()
    })
}

fn template_changed_fields(
    previous: &StockTemplateDetail,
    updated: &StockTemplateDetail,
    fields_replaced: bool,
) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if previous.template.name != updated.template.name {
        fields.push("name");
    }
    if previous.template.description != updated.template.description {
        fields.push("description");
    }
    if fields_replaced {
        fields.push("fields");
    }

    fields
}

async fn insert_template_on_connection<C>(
    connection: &C,
    name: &str,
    description: Option<String>,
) -> Result<stock_template::Model, DbErr>
where
    C: ConnectionTrait,
{
    let now = sqlite_now(connection).await?;
    let active_model = stock_template::ActiveModel {
        name: Set(name.to_owned()),
        description: Set(description),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        deleted_at: Set(None),
        ..Default::default()
    };
    let result = stock_template::Entity::insert(active_model)
        .exec(connection)
        .await?;

    stock_template::Entity::find_by_id(result.last_insert_id)
        .one(connection)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("created stock template".to_owned()))
}

async fn replace_template_fields_on_connection<C>(
    connection: &C,
    template_id: i64,
    fields: &[TemplateFieldInput],
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    connection
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "DELETE FROM stock_template_fields WHERE template_id = ?",
            [template_id.into()],
        ))
        .await?;

    for field in fields {
        validate_repository_input(field)?;
        let now = sqlite_now(connection).await?;
        let active_model = stock_template_field::ActiveModel {
            template_id: Set(template_id),
            field_name: Set(field.field_name.clone()),
            field_type: Set(field.field_type.clone()),
            required: Set(bool_to_sqlite(field.required)),
            searchable: Set(bool_to_sqlite(field.searchable)),
            options_json: Set(field.options_json.clone()),
            default_value: Set(field.default_value.clone()),
            sort_order: Set(field.sort_order),
            created_at: Set(now.clone()),
            updated_at: Set(now),
            ..Default::default()
        };
        stock_template_field::Entity::insert(active_model)
            .exec(connection)
            .await?;
    }

    Ok(())
}

async fn list_template_fields_on_connection<C>(
    connection: &C,
    template_id: i64,
) -> Result<Vec<stock_template_field::Model>, DbErr>
where
    C: ConnectionTrait,
{
    stock_template_field::Entity::find()
        .filter(stock_template_field::Column::TemplateId.eq(template_id))
        .order_by_asc(stock_template_field::Column::SortOrder)
        .order_by_asc(stock_template_field::Column::Id)
        .all(connection)
        .await
}

fn bool_to_sqlite(value: bool) -> i32 {
    if value {
        1
    } else {
        0
    }
}
