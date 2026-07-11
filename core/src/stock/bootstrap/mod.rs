//! 库存默认数据启动补齐。
//!
//! 本模块属于 stock 业务层，分别补齐分类、物品属性预设、入库模板和默认库位；
//! 它不覆盖或恢复用户已经修改、软删除的同名记录。

mod specs;

use sea_orm::{DatabaseConnection, DbErr};
use std::{error::Error, fmt};

use crate::persistence::repository::{
    CreateInboundTemplate, CreateItemAttributeTemplate, CreateItemCategory, StockRepository,
};
use specs::{
    category_input, inbound_template_input, item_template_input, DEFAULT_CATEGORIES,
    DEFAULT_INBOUND_TEMPLATES, DEFAULT_ITEM_TEMPLATES,
};

/// 库存默认数据启动补齐失败。
#[derive(Debug)]
pub enum StockBootstrapError {
    /// 读取或写入库存默认数据失败。
    Database(DbErr),
}

impl fmt::Display for StockBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(source) => write!(f, "failed to bootstrap stock defaults: {source}"),
        }
    }
}
impl Error for StockBootstrapError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Database(source) => Some(source),
        }
    }
}
impl From<DbErr> for StockBootstrapError {
    fn from(source: DbErr) -> Self {
        Self::Database(source)
    }
}

/// 启动时补齐库存默认数据；所有模板均按独立业务表写入。
pub(crate) async fn bootstrap_default_templates(
    database: &DatabaseConnection,
) -> Result<(), StockBootstrapError> {
    let repository = StockRepository::new(database);
    repository.ensure_default_location().await?;
    for category in DEFAULT_CATEGORIES {
        if !repository.item_category_name_exists(category.name).await? {
            repository
                .create_item_category(category_input(category), None)
                .await?;
        }
    }
    for template in DEFAULT_INBOUND_TEMPLATES {
        if !repository
            .inbound_template_name_exists(template.name)
            .await?
        {
            repository
                .create_inbound_template(inbound_template_input(template), None)
                .await?;
        }
    }
    let inbound_templates = repository.list_active_inbound_templates().await?;
    for template in DEFAULT_ITEM_TEMPLATES {
        if repository
            .item_attribute_template_name_exists(template.name)
            .await?
        {
            continue;
        }
        let default_inbound_template_id = template.default_inbound_template_name.and_then(|name| {
            inbound_templates
                .iter()
                .find(|entry| entry.template.name == name)
                .map(|entry| entry.template.id)
        });
        repository
            .create_item_attribute_template(
                item_template_input(template, default_inbound_template_id),
                None,
            )
            .await?;
    }
    Ok(())
}

#[allow(dead_code)]
fn _assert_inputs(_: CreateItemCategory, _: CreateInboundTemplate, _: CreateItemAttributeTemplate) {
}
