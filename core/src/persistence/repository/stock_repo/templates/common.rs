//! 模板仓储共享的字段读写与审计辅助函数。
//!
//! 本模块属于 core 持久化层，只复用两类模板相同的机械流程，不决定模板业务语义。

use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set, Statement,
};
use serde_json::json;

use super::super::{
    common::insert_audit_event_on_connection, InboundTemplateDetail, ItemAttributeTemplateDetail,
    TemplateFieldInput,
};
use crate::persistence::{
    entity::{
        inbound_template, inbound_template_field, item_attribute_template,
        item_attribute_template_field,
    },
    repository::{time::sqlite_now, validation::validate_repository_input},
};

/// 插入入库模板主记录，字段由调用方在同一事务中继续写入。
pub(super) async fn insert_inbound_template<C>(
    connection: &C,
    name: &str,
    description: Option<String>,
) -> Result<inbound_template::Model, DbErr>
where
    C: ConnectionTrait,
{
    let now = sqlite_now(connection).await?;
    let result = inbound_template::Entity::insert(inbound_template::ActiveModel {
        name: Set(name.to_owned()),
        description: Set(description),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        deleted_at: Set(None),
        ..Default::default()
    })
    .exec(connection)
    .await?;
    inbound_template::Entity::find_by_id(result.last_insert_id)
        .one(connection)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("created inbound template".to_owned()))
}

/// 插入物品属性模板主记录，预设字段由调用方继续写入。
pub(super) async fn insert_item_attribute_template<C>(
    connection: &C,
    name: &str,
    description: Option<String>,
    default_inbound_template_id: Option<i64>,
) -> Result<item_attribute_template::Model, DbErr>
where
    C: ConnectionTrait,
{
    let now = sqlite_now(connection).await?;
    let result = item_attribute_template::Entity::insert(item_attribute_template::ActiveModel {
        name: Set(name.to_owned()),
        description: Set(description),
        default_inbound_template_id: Set(default_inbound_template_id),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        deleted_at: Set(None),
        ..Default::default()
    })
    .exec(connection)
    .await?;
    item_attribute_template::Entity::find_by_id(result.last_insert_id)
        .one(connection)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("created item attribute template".to_owned()))
}

/// 整体替换入库模板字段；调用方负责提供事务连接。
pub(super) async fn replace_inbound_fields<C>(
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
            "DELETE FROM stock_inbound_template_fields WHERE template_id = ?",
            [template_id.into()],
        ))
        .await?;
    for field in fields {
        validate_repository_input(field)?;
        let now = sqlite_now(connection).await?;
        inbound_template_field::Entity::insert(inbound_template_field::ActiveModel {
            template_id: Set(template_id),
            field_name: Set(field.field_name.clone()),
            field_type: Set(field.field_type.clone()),
            required: Set(i32::from(field.required)),
            searchable: Set(i32::from(field.searchable)),
            options_json: Set(field.options_json.clone()),
            default_value: Set(field.default_value.clone()),
            sort_order: Set(field.sort_order),
            created_at: Set(now.clone()),
            updated_at: Set(now),
            ..Default::default()
        })
        .exec(connection)
        .await?;
    }
    Ok(())
}

/// 整体替换物品属性模板字段；调用方负责提供事务连接。
pub(super) async fn replace_item_attribute_fields<C>(
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
            "DELETE FROM stock_item_attribute_template_fields WHERE template_id = ?",
            [template_id.into()],
        ))
        .await?;
    for field in fields {
        validate_repository_input(field)?;
        let now = sqlite_now(connection).await?;
        item_attribute_template_field::Entity::insert(item_attribute_template_field::ActiveModel {
            template_id: Set(template_id),
            field_name: Set(field.field_name.clone()),
            field_type: Set(field.field_type.clone()),
            required: Set(i32::from(field.required)),
            searchable: Set(i32::from(field.searchable)),
            options_json: Set(field.options_json.clone()),
            default_value: Set(field.default_value.clone()),
            sort_order: Set(field.sort_order),
            created_at: Set(now.clone()),
            updated_at: Set(now),
            ..Default::default()
        })
        .exec(connection)
        .await?;
    }
    Ok(())
}

/// 在指定连接上查询有效入库模板详情。
pub(super) async fn find_active_inbound_template<C>(
    connection: &C,
    id: i64,
) -> Result<Option<InboundTemplateDetail>, DbErr>
where
    C: ConnectionTrait,
{
    let Some(template) = inbound_template::Entity::find_by_id(id)
        .filter(inbound_template::Column::DeletedAt.is_null())
        .one(connection)
        .await?
    else {
        return Ok(None);
    };
    let fields = list_inbound_fields(connection, id).await?;
    Ok(Some(InboundTemplateDetail { template, fields }))
}

/// 在指定连接上查询有效物品属性模板详情。
pub(super) async fn find_active_item_attribute_template<C>(
    connection: &C,
    id: i64,
) -> Result<Option<ItemAttributeTemplateDetail>, DbErr>
where
    C: ConnectionTrait,
{
    let Some(template) = item_attribute_template::Entity::find_by_id(id)
        .filter(item_attribute_template::Column::DeletedAt.is_null())
        .one(connection)
        .await?
    else {
        return Ok(None);
    };
    let fields = list_item_attribute_fields(connection, id).await?;
    Ok(Some(ItemAttributeTemplateDetail { template, fields }))
}

/// 查询入库模板字段并保持稳定排序。
pub(super) async fn list_inbound_fields<C>(
    connection: &C,
    template_id: i64,
) -> Result<Vec<inbound_template_field::Model>, DbErr>
where
    C: ConnectionTrait,
{
    inbound_template_field::Entity::find()
        .filter(inbound_template_field::Column::TemplateId.eq(template_id))
        .order_by_asc(inbound_template_field::Column::SortOrder)
        .order_by_asc(inbound_template_field::Column::Id)
        .all(connection)
        .await
}

/// 查询物品属性模板字段并保持稳定排序。
pub(super) async fn list_item_attribute_fields<C>(
    connection: &C,
    template_id: i64,
) -> Result<Vec<item_attribute_template_field::Model>, DbErr>
where
    C: ConnectionTrait,
{
    item_attribute_template_field::Entity::find()
        .filter(item_attribute_template_field::Column::TemplateId.eq(template_id))
        .order_by_asc(item_attribute_template_field::Column::SortOrder)
        .order_by_asc(item_attribute_template_field::Column::Id)
        .all(connection)
        .await
}

/// 将入库模板实体字段投影为可复制的仓储输入。
pub(super) fn inbound_field_inputs(
    fields: &[inbound_template_field::Model],
) -> Vec<TemplateFieldInput> {
    fields.iter().map(inbound_field_input).collect()
}

fn inbound_field_input(field: &inbound_template_field::Model) -> TemplateFieldInput {
    TemplateFieldInput {
        field_name: field.field_name.clone(),
        field_type: field.field_type.clone(),
        required: field.required != 0,
        searchable: field.searchable != 0,
        options_json: field.options_json.clone(),
        default_value: field.default_value.clone(),
        sort_order: field.sort_order,
    }
}

/// 将物品属性模板实体字段投影为可复制的仓储输入。
pub(super) fn item_attribute_field_inputs(
    fields: &[item_attribute_template_field::Model],
) -> Vec<TemplateFieldInput> {
    fields
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
        .collect()
}

/// 写入模板审计摘要；不记录模板字段的潜在敏感值。
pub(super) async fn audit_template_change<C>(
    connection: &C,
    user_id: Option<i64>,
    entity_type: &str,
    entity_id: i64,
    action: &str,
    name: &str,
    field_count: usize,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    if let Some(user_id) = user_id {
        insert_audit_event_on_connection(
            connection,
            Some(user_id),
            entity_type,
            Some(entity_id),
            action,
            Some(json!({ "name": name, "field_count": field_count }).to_string()),
        )
        .await?;
    }
    Ok(())
}
