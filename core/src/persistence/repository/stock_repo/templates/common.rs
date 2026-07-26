//! 物品属性模板仓储共享的字段读写与审计辅助函数。
//!
//! 本模块属于 core 持久化层，只承载模板字段写入与审计的机械流程，不决定模板业务语义。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DbErr, EntityTrait,
    QueryFilter, QueryOrder, Set, Statement,
};
use serde_json::json;

use super::super::{
    common::insert_audit_event_on_connection, ItemAttributeTemplateDetail, TemplateFieldInput,
};
use crate::persistence::{
    entity::{item_attribute_definition, item_attribute_template},
    repository::{time::sqlite_now, validation::validate_repository_input},
};

/// 插入物品属性模板主记录，预设字段由调用方继续写入。
pub(super) async fn insert_item_attribute_template<C>(
    connection: &C,
    name: &str,
    description: Option<String>,
) -> Result<item_attribute_template::Model, DbErr>
where
    C: ConnectionTrait,
{
    let now = sqlite_now(connection).await?;
    let result = item_attribute_template::Entity::insert(item_attribute_template::ActiveModel {
        name: Set(name.to_owned()),
        description: Set(description),
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

/// 整体替换物品属性模板字段；调用方负责提供事务连接。
pub(super) async fn replace_item_attribute_fields<C>(
    connection: &C,
    template_id: i64,
    fields: &[TemplateFieldInput],
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let retained_ids = fields
        .iter()
        .filter_map(|field| field.definition_id)
        .collect::<Vec<_>>();
    let mut delete_values = vec![template_id.into()];
    let delete_sql = if retained_ids.is_empty() {
        "DELETE FROM stock_item_attribute_definitions WHERE template_id = ?".to_owned()
    } else {
        delete_values.extend(retained_ids.iter().copied().map(Into::into));
        format!(
            "DELETE FROM stock_item_attribute_definitions WHERE template_id = ? AND id NOT IN ({})",
            vec!["?"; retained_ids.len()].join(", ")
        )
    };
    connection
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            delete_sql,
            delete_values,
        ))
        .await?;
    for field in fields {
        validate_repository_input(field)?;
        let now = sqlite_now(connection).await?;
        if let Some(definition_id) = field.definition_id {
            let definition = item_attribute_definition::Entity::find_by_id(definition_id)
                .one(connection)
                .await?
                .ok_or_else(|| DbErr::Custom("item template definition missing".to_owned()))?;
            if definition.template_id != Some(template_id) || definition.owner_item_id.is_some() {
                return Err(DbErr::Custom(
                    "item template definition ownership mismatch".to_owned(),
                ));
            }
            let mut active: item_attribute_definition::ActiveModel = definition.into();
            active.field_name = Set(field.field_name.clone());
            active.field_type = Set(field.field_type.clone());
            active.required = Set(i32::from(field.required));
            active.searchable = Set(i32::from(field.searchable));
            active.catalog_visible = Set(i32::from(field.catalog_visible));
            active.options_json = Set(field.options_json.clone());
            active.default_value = Set(field.default_value.clone());
            active.unit_mode = Set(field.unit_mode.clone());
            active.fixed_unit = Set(field.fixed_unit.clone());
            active.unit_options_json = Set(field.unit_options_json.clone());
            active.sort_order = Set(field.sort_order);
            active.updated_at = Set(now);
            active.update(connection).await?;
            continue;
        }
        item_attribute_definition::Entity::insert(item_attribute_definition::ActiveModel {
            template_id: Set(Some(template_id)),
            owner_item_id: Set(None),
            field_name: Set(field.field_name.clone()),
            field_type: Set(field.field_type.clone()),
            required: Set(i32::from(field.required)),
            searchable: Set(i32::from(field.searchable)),
            catalog_visible: Set(i32::from(field.catalog_visible)),
            options_json: Set(field.options_json.clone()),
            default_value: Set(field.default_value.clone()),
            unit_mode: Set(field.unit_mode.clone()),
            fixed_unit: Set(field.fixed_unit.clone()),
            unit_options_json: Set(field.unit_options_json.clone()),
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

/// 查询物品属性模板字段并保持稳定排序。
pub(super) async fn list_item_attribute_fields<C>(
    connection: &C,
    template_id: i64,
) -> Result<Vec<item_attribute_definition::Model>, DbErr>
where
    C: ConnectionTrait,
{
    item_attribute_definition::Entity::find()
        .filter(item_attribute_definition::Column::TemplateId.eq(template_id))
        .order_by_asc(item_attribute_definition::Column::SortOrder)
        .order_by_asc(item_attribute_definition::Column::Id)
        .all(connection)
        .await
}

/// 将物品属性模板实体字段投影为可复制的仓储输入。
pub(super) fn item_attribute_field_inputs(
    fields: &[item_attribute_definition::Model],
) -> Vec<TemplateFieldInput> {
    fields
        .iter()
        .map(|field| TemplateFieldInput {
            definition_id: None,
            field_name: field.field_name.clone(),
            field_type: field.field_type.clone(),
            required: field.required != 0,
            searchable: field.searchable != 0,
            catalog_visible: field.catalog_visible != 0,
            options_json: field.options_json.clone(),
            default_value: field.default_value.clone(),
            unit_mode: field.unit_mode.clone(),
            fixed_unit: field.fixed_unit.clone(),
            unit_options_json: field.unit_options_json.clone(),
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
