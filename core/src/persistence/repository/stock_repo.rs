//! stock 模块 repository。
//!
//! 本模块属于 `core` 的持久化层，封装库存物品、后续模板和出入库查询。
//! handler 和 service 不应直接拼接 `stock_*` 表结构。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr,
    EntityTrait, QueryFilter, QueryOrder, Set, Statement, TransactionTrait, Value,
};
use std::collections::HashSet;
use winestock_shared::validation::{validate_not_blank, validate_optional_not_blank};

use crate::persistence::{
    entity::{stock_item, stock_template, stock_template_field},
    repository::{time::sqlite_now, validation::validate_repository_input},
};

/// 创建库存物品的仓储输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct CreateStockItem {
    /// 物品名称，裁剪后不能为空。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 物品 SKU，未软删除记录内唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub sku: String,

    /// 关联模板 ID；为空表示暂不关联模板。
    #[garde(skip)]
    pub category_id: Option<i64>,

    /// 计量单位，裁剪后不能为空。
    #[garde(length(min = 1, max = 32), custom(validate_not_blank))]
    pub unit: String,

    /// 物品描述。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 参考单价，不允许为负。
    #[garde(skip)]
    pub default_price: Option<f64>,

    /// 再订货点，不允许为负。
    #[garde(skip)]
    pub reorder_point: Option<f64>,
}

/// 更新库存物品的仓储输入；为空字段表示不修改。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct UpdateStockItem {
    /// 物品名称，存在时裁剪后不能为空。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,

    /// 物品 SKU，存在时裁剪后不能为空且未软删除记录内唯一。
    #[garde(length(min = 1, max = 64), custom(validate_optional_not_blank))]
    pub sku: Option<String>,

    /// 关联模板 ID；外层 Option 表示是否修改，内层 Option 表示是否清空。
    #[garde(skip)]
    pub category_id: Option<Option<i64>>,

    /// 计量单位，存在时裁剪后不能为空。
    #[garde(length(min = 1, max = 32), custom(validate_optional_not_blank))]
    pub unit: Option<String>,

    /// 物品描述；外层 Option 表示是否修改，内层 Option 表示是否清空。
    #[garde(skip)]
    pub description: Option<Option<String>>,

    /// 参考单价；外层 Option 表示是否修改，内层 Option 表示是否清空。
    #[garde(skip)]
    pub default_price: Option<Option<f64>>,

    /// 再订货点；外层 Option 表示是否修改，内层 Option 表示是否清空。
    #[garde(skip)]
    pub reorder_point: Option<Option<f64>>,
}

/// 库存物品分页查询条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListStockItems {
    /// 页码，从 1 开始。
    pub page: u64,

    /// 每页数量，服务层负责限制最大值。
    pub page_size: u64,

    /// 名称或 SKU 模糊搜索关键字。
    pub search: Option<String>,

    /// 按模板 ID 筛选。
    pub category_id: Option<i64>,
}

/// 创建模板字段定义的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct TemplateFieldInput {
    /// 字段名称，同一模板内不能重复。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub field_name: String,

    /// 字段类型稳定代码。
    #[garde(length(min = 1, max = 32), custom(validate_not_blank))]
    pub field_type: String,

    /// 是否必填。
    #[garde(skip)]
    pub required: bool,

    /// 是否可用于搜索。
    #[garde(skip)]
    pub searchable: bool,

    /// 候选值 JSON，仅 `select` 字段使用。
    #[garde(length(min = 1, max = 4096), custom(validate_optional_not_blank))]
    pub options_json: Option<String>,

    /// 默认值。
    #[garde(length(min = 1, max = 256), custom(validate_optional_not_blank))]
    pub default_value: Option<String>,

    /// 字段排序，从 0 开始。
    #[garde(range(min = 0))]
    pub sort_order: i32,
}

/// 创建库存模板的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct CreateStockTemplate {
    /// 模板名称，未软删除记录内唯一。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 模板说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 模板字段定义。
    #[garde(dive)]
    pub fields: Vec<TemplateFieldInput>,
}

/// 更新库存模板的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct UpdateStockTemplate {
    /// 模板名称，存在时未软删除记录内唯一。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,

    /// 模板说明；外层 Option 表示是否修改，内层 Option 表示是否清空。
    #[garde(skip)]
    pub description: Option<Option<String>>,

    /// 模板字段定义；存在时整体替换旧字段。
    #[garde(skip)]
    pub fields: Option<Vec<TemplateFieldInput>>,
}

/// 库存模板详情，包含模板基础资料和字段定义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StockTemplateDetail {
    /// 模板基础资料。
    pub template: stock_template::Model,

    /// 模板字段定义，按 `sort_order, id` 排序。
    pub fields: Vec<stock_template_field::Model>,
}

/// 创建入库单明细的仓储输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct CreateInboundOrderItem {
    /// 入库物品 ID。
    #[garde(range(min = 1))]
    pub item_id: i64,

    /// 入库数量，必须大于 0。
    #[garde(custom(validate_positive_f64))]
    pub quantity: f64,

    /// 入库单价，不允许为负。
    #[garde(range(min = 0.0))]
    pub unit_price: f64,

    /// 存储库位。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub location: Option<String>,

    /// 外部批次号；为空时审批阶段生成内部批次号。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub batch_no: Option<String>,

    /// 有效期字符串；首版仅保存调用方传入的日期文本。
    #[garde(length(min = 1, max = 64), custom(validate_optional_not_blank))]
    pub expires_at: Option<String>,

    /// 模板扩展属性 JSON 字符串。
    #[garde(length(min = 1, max = 8192), custom(validate_optional_not_blank))]
    pub ext_attributes_json: Option<String>,
}

/// 创建入库单的仓储输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct CreateInboundOrder {
    /// 入库来源。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub source: String,

    /// 备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 创建人用户 ID。
    #[garde(skip)]
    pub created_by_user_id: Option<i64>,

    /// 入库明细。
    #[garde(dive)]
    pub items: Vec<CreateInboundOrderItem>,
}

/// 入库单分页查询条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListInboundOrders {
    /// 页码，从 1 开始。
    pub page: u64,

    /// 每页数量。
    pub page_size: u64,

    /// 按物品 ID 筛选。
    pub item_id: Option<i64>,

    /// 创建时间起点。
    pub date_from: Option<String>,

    /// 创建时间终点。
    pub date_to: Option<String>,
}

/// 入库单主表读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InboundOrderRecord {
    /// 入库单 ID。
    pub id: i64,

    /// 入库来源。
    pub source: String,

    /// 单据状态。
    pub status: String,

    /// 备注。
    pub notes: Option<String>,

    /// 创建人用户 ID。
    pub created_by_user_id: Option<i64>,

    /// 审批人用户 ID。
    pub approved_by_user_id: Option<i64>,

    /// 拒绝人用户 ID。
    pub rejected_by_user_id: Option<i64>,

    /// 创建时间。
    pub created_at: String,

    /// 更新时间。
    pub updated_at: String,

    /// 审批时间。
    pub approved_at: Option<String>,

    /// 拒绝时间。
    pub rejected_at: Option<String>,
}

/// 入库单明细读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InboundOrderItemRecord {
    /// 明细 ID。
    pub id: i64,

    /// 所属入库单 ID。
    pub order_id: i64,

    /// 物品 ID。
    pub item_id: i64,

    /// 入库数量。
    pub quantity: f64,

    /// 入库单价。
    pub unit_price: f64,

    /// 存储库位。
    pub location: Option<String>,

    /// 批次号。
    pub batch_no: Option<String>,

    /// 有效期。
    pub expires_at: Option<String>,

    /// 模板扩展属性 JSON。
    pub ext_attributes_json: Option<String>,

    /// 创建时间。
    pub created_at: String,
}

/// 入库单详情读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InboundOrderDetail {
    /// 入库单主表记录。
    pub order: InboundOrderRecord,

    /// 入库单明细。
    pub items: Vec<InboundOrderItemRecord>,
}

/// 创建出库单明细的仓储输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct CreateOutboundOrderItem {
    /// 出库物品 ID。
    #[garde(range(min = 1))]
    pub item_id: i64,

    /// 出库数量，必须大于 0。
    #[garde(custom(validate_positive_f64))]
    pub quantity: f64,

    /// 指定扣减批次；为空时审批阶段按 FIFO 扣减。
    #[garde(skip)]
    pub batch_id: Option<i64>,

    /// 出库库位。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub location: Option<String>,
}

/// 创建出库单的仓储输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct CreateOutboundOrder {
    /// 出库去向。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub destination: String,

    /// 备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 创建人用户 ID。
    #[garde(skip)]
    pub created_by_user_id: Option<i64>,

    /// 出库明细。
    #[garde(dive)]
    pub items: Vec<CreateOutboundOrderItem>,
}

/// 出库单分页查询条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListOutboundOrders {
    /// 页码，从 1 开始。
    pub page: u64,

    /// 每页数量。
    pub page_size: u64,

    /// 按物品 ID 筛选。
    pub item_id: Option<i64>,

    /// 创建时间起点。
    pub date_from: Option<String>,

    /// 创建时间终点。
    pub date_to: Option<String>,
}

/// 出库单主表读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OutboundOrderRecord {
    /// 出库单 ID。
    pub id: i64,

    /// 出库去向。
    pub destination: String,

    /// 单据状态。
    pub status: String,

    /// 备注。
    pub notes: Option<String>,

    /// 创建人用户 ID。
    pub created_by_user_id: Option<i64>,

    /// 审批人用户 ID。
    pub approved_by_user_id: Option<i64>,

    /// 拒绝人用户 ID。
    pub rejected_by_user_id: Option<i64>,

    /// 创建时间。
    pub created_at: String,

    /// 更新时间。
    pub updated_at: String,

    /// 审批时间。
    pub approved_at: Option<String>,

    /// 拒绝时间。
    pub rejected_at: Option<String>,
}

/// 出库单明细读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OutboundOrderItemRecord {
    /// 明细 ID。
    pub id: i64,

    /// 所属出库单 ID。
    pub order_id: i64,

    /// 物品 ID。
    pub item_id: i64,

    /// 出库数量。
    pub quantity: f64,

    /// 指定扣减批次。
    pub batch_id: Option<i64>,

    /// 出库库位。
    pub location: Option<String>,

    /// 创建时间。
    pub created_at: String,
}

/// 出库单详情读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OutboundOrderDetail {
    /// 出库单主表记录。
    pub order: OutboundOrderRecord,

    /// 出库单明细。
    pub items: Vec<OutboundOrderItemRecord>,
}

/// 看板总览聚合读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DashboardOverviewRecord {
    /// 未软删除的库存物品种类数。
    pub total_items: i64,

    /// 当前所有有效批次的剩余总数量。
    pub total_quantity: f64,

    /// 当前所有有效批次按批次成本计算的库存总价值。
    pub total_value: f64,

    /// 最近三天已审批入库流水总数量。
    pub inbound_3d: f64,

    /// 最近三天已审批出库流水总数量。
    pub outbound_3d: f64,

    /// 当前有库存但超过阈值天数未发生出入库流水的物品。
    pub slow_moving_items: Vec<SlowMovingStockItemRecord>,
}

/// 呆滞料聚合读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SlowMovingStockItemRecord {
    /// 物品 ID。
    pub item_id: i64,

    /// 物品名称。
    pub item_name: String,

    /// 当前剩余库存量。
    pub quantity: f64,

    /// 当前库存价值。
    pub value: f64,

    /// 最近一次出入库流水距今天数。
    pub days_since_last_movement: i64,
}

/// 每日出入库趋势聚合读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DailyMovementTrendRecord {
    /// 日期，格式为 `YYYY-MM-DD`。
    pub date: String,

    /// 当日已审批入库数量。
    pub inbound_quantity: f64,

    /// 当日已审批出库数量。
    pub outbound_quantity: f64,
}

/// 绑定替代料的仓储输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct BindStockSubstitute {
    /// 替代料物品 ID。
    #[garde(range(min = 1))]
    pub substitute_item_id: i64,

    /// 替代优先级，数值越小越优先。
    #[garde(range(min = 1))]
    pub priority: i32,

    /// 兼容性备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,
}

/// 替代料关系读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StockSubstituteRecord {
    /// 主物品 ID。
    pub item_id: i64,

    /// 替代料物品 ID。
    pub substitute_item_id: i64,

    /// 替代料物品名称。
    pub substitute_item_name: String,

    /// 替代料当前库存量。
    pub quantity: f64,

    /// 替代优先级。
    pub priority: i32,

    /// 兼容性备注。
    pub notes: Option<String>,

    /// 创建人用户 ID。
    pub created_by_user_id: Option<i64>,

    /// 创建时间。
    pub created_at: String,
}

/// 审计事件分页查询条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListAuditEvents {
    /// 页码，从 1 开始。
    pub page: u64,

    /// 每页数量。
    pub page_size: u64,

    /// 按实体类型筛选。
    pub entity_type: Option<String>,

    /// 按实体 ID 筛选。
    pub entity_id: Option<i64>,

    /// 按动作筛选。
    pub action: Option<String>,

    /// 按操作人用户 ID 筛选。
    pub user_id: Option<i64>,

    /// 操作时间起点。
    pub date_from: Option<String>,

    /// 操作时间终点。
    pub date_to: Option<String>,
}

/// 审计事件读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AuditEventRecord {
    /// 审计事件 ID。
    pub id: i64,

    /// 操作时间。
    pub timestamp: String,

    /// 操作人用户 ID。
    pub user_id: Option<i64>,

    /// 操作人用户名。
    pub username: Option<String>,

    /// 被操作实体类型。
    pub entity_type: String,

    /// 被操作实体 ID。
    pub entity_id: Option<i64>,

    /// 操作动作。
    pub action: String,

    /// 事件详情 JSON 字符串。
    pub details_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct StockBatchForDeduction {
    id: i64,
    remaining_quantity: f64,
    unit_cost: f64,
}

/// 分页查询结果。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Page<T> {
    /// 当前页数据。
    pub items: Vec<T>,

    /// 满足条件的总记录数。
    pub total: u64,
}

/// stock 仓储层封装库存领域持久化语义。
pub(crate) struct StockRepository<'db, C = DatabaseConnection>
where
    C: ConnectionTrait,
{
    database: &'db C,
}

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建绑定到同一个 SeaORM 连接的 stock 仓储。
    pub(crate) fn new(database: &'db C) -> Self {
        Self { database }
    }

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

    /// 创建模板和字段定义；父模板与字段必须在同一事务内写入。
    pub(crate) async fn create_template(
        &self,
        input: CreateStockTemplate,
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

    /// 更新模板和可选字段定义；字段替换必须与模板更新时间在同一事务内完成。
    pub(crate) async fn update_template(
        &self,
        id: i64,
        input: UpdateStockTemplate,
    ) -> Result<Option<StockTemplateDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let Some(template) = stock_template::Entity::find_by_id(id)
            .filter(stock_template::Column::DeletedAt.is_null())
            .one(self.database)
            .await?
        else {
            return Ok(None);
        };

        let transaction = self.database.begin().await?;
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

        transaction.commit().await?;
        self.find_active_template_by_id(id).await
    }

    /// 复制模板及其字段定义，并使用新名称创建未删除模板。
    pub(crate) async fn copy_template(
        &self,
        id: i64,
        new_name: String,
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

    /// 软删除模板；调用方必须先确认未被有效物品引用。
    pub(crate) async fn soft_delete_template(&self, id: i64) -> Result<bool, DbErr> {
        let Some(template) = stock_template::Entity::find_by_id(id)
            .filter(stock_template::Column::DeletedAt.is_null())
            .one(self.database)
            .await?
        else {
            return Ok(false);
        };
        let now = sqlite_now(self.database).await?;
        let mut active_model: stock_template::ActiveModel = template.into();
        active_model.updated_at = Set(now.clone());
        active_model.deleted_at = Set(Some(now));
        active_model.update(self.database).await?;

        Ok(true)
    }

    /// 创建 pending 入库单和明细；创建阶段不改变库存。
    pub(crate) async fn create_inbound_order(
        &self,
        input: CreateInboundOrder,
    ) -> Result<InboundOrderDetail, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        if input.items.is_empty() {
            return Err(DbErr::Custom(
                "inbound order items must not be empty".to_owned(),
            ));
        }

        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        let result = transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO stock_inbound_orders
                    (source, status, notes, created_by_user_id, created_at, updated_at)
                VALUES (?, 'pending', ?, ?, ?, ?)
                "#,
                vec![
                    input.source.clone().into(),
                    input.notes.into(),
                    input.created_by_user_id.into(),
                    now.clone().into(),
                    now.clone().into(),
                ],
            ))
            .await?;
        let order_id = i64::try_from(result.last_insert_id())
            .map_err(|_| DbErr::Custom("inbound order id overflow".to_owned()))?;

        for item in &input.items {
            validate_repository_input(item)?;
            transaction
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    r#"
                    INSERT INTO stock_inbound_order_items
                        (order_id, item_id, quantity, unit_price, location, batch_no, expires_at, ext_attributes_json, created_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    vec![
                        order_id.into(),
                        item.item_id.into(),
                        item.quantity.into(),
                        item.unit_price.into(),
                        item.location.clone().into(),
                        item.batch_no.clone().into(),
                        item.expires_at.clone().into(),
                        item.ext_attributes_json.clone().into(),
                        now.clone().into(),
                    ],
                ))
                .await?;
        }
        insert_audit_event_on_connection(
            &transaction,
            input.created_by_user_id,
            "inbound",
            Some(order_id),
            "created",
            Some(format!(
                r#"{{"source":{},"item_count":{}}}"#,
                json_string(&input.source),
                input.items.len()
            )),
        )
        .await?;
        transaction.commit().await?;

        self.find_inbound_order_by_id(order_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created inbound order".to_owned()))
    }

    /// 查询入库单详情。
    pub(crate) async fn find_inbound_order_by_id(
        &self,
        id: i64,
    ) -> Result<Option<InboundOrderDetail>, DbErr> {
        let Some(order) = self.find_inbound_order_record_by_id(id).await? else {
            return Ok(None);
        };
        let items = list_inbound_items_on_connection(self.database, id).await?;

        Ok(Some(InboundOrderDetail { order, items }))
    }

    /// 分页查询入库单，支持物品和创建时间筛选。
    pub(crate) async fn list_inbound_orders(
        &self,
        input: ListInboundOrders,
    ) -> Result<Page<InboundOrderDetail>, DbErr> {
        let limit = input.page_size as i64;
        let offset = ((input.page.saturating_sub(1)) * input.page_size) as i64;
        let total = self.count_inbound_orders(&input).await?;
        let rows = self.query_inbound_orders(&input, limit, offset).await?;
        let mut items = Vec::with_capacity(rows.len());
        for order in rows {
            let order_items = list_inbound_items_on_connection(self.database, order.id).await?;
            items.push(InboundOrderDetail {
                order,
                items: order_items,
            });
        }

        Ok(Page { items, total })
    }

    /// 审批 pending 入库单；状态、批次、库存流水和审计事件必须在同一事务内完成。
    pub(crate) async fn approve_inbound_order(
        &self,
        id: i64,
        user_id: Option<i64>,
    ) -> Result<Option<InboundOrderDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        let Some(order) = self.find_inbound_order_record_by_id(id).await? else {
            return Ok(None);
        };
        if order.status != "pending" {
            return Err(DbErr::Custom("inbound order is not pending".to_owned()));
        }
        let order_items = list_inbound_items_on_connection(self.database, id).await?;
        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_inbound_orders
                SET status = 'approved',
                    approved_by_user_id = ?,
                    approved_at = ?,
                    updated_at = ?
                WHERE id = ? AND status = 'pending'
                "#,
                vec![
                    user_id.into(),
                    now.clone().into(),
                    now.clone().into(),
                    id.into(),
                ],
            ))
            .await?;

        for item in &order_items {
            let batch_no = item
                .batch_no
                .clone()
                .unwrap_or_else(|| format!("IN-{id}-{}", item.id));
            let batch_result = transaction
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    r#"
                    INSERT INTO stock_batches
                        (item_id, inbound_order_item_id, batch_no, location, initial_quantity, remaining_quantity, unit_cost, received_at, expires_at, created_at, updated_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    vec![
                        item.item_id.into(),
                        item.id.into(),
                        batch_no.into(),
                        item.location.clone().into(),
                        item.quantity.into(),
                        item.quantity.into(),
                        item.unit_price.into(),
                        now.clone().into(),
                        item.expires_at.clone().into(),
                        now.clone().into(),
                        now.clone().into(),
                    ],
                ))
                .await?;
            let balance_after =
                current_item_quantity_on_connection(&transaction, item.item_id).await?;
            transaction
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    r#"
                    INSERT INTO stock_movements
                        (item_id, batch_id, movement_type, quantity_delta, unit_cost, balance_after, inbound_order_item_id, created_by_user_id, created_at)
                    VALUES (?, ?, 'inbound', ?, ?, ?, ?, ?, ?)
                    "#,
                    vec![
                        item.item_id.into(),
                        i64::try_from(batch_result.last_insert_id())
                            .map_err(|_| DbErr::Custom("stock batch id overflow".to_owned()))?
                            .into(),
                        item.quantity.into(),
                        item.unit_price.into(),
                        balance_after.into(),
                        item.id.into(),
                        user_id.into(),
                        now.clone().into(),
                    ],
                ))
                .await?;
        }
        insert_audit_event_on_connection(
            &transaction,
            user_id,
            "inbound",
            Some(id),
            "approved",
            Some(format!(r#"{{"item_count":{}}}"#, order_items.len())),
        )
        .await?;
        transaction.commit().await?;

        self.find_inbound_order_by_id(id).await
    }

    /// 拒绝 pending 入库单；拒绝不改变库存。
    pub(crate) async fn reject_inbound_order(
        &self,
        id: i64,
        user_id: Option<i64>,
    ) -> Result<Option<InboundOrderDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        let Some(order) = self.find_inbound_order_record_by_id(id).await? else {
            return Ok(None);
        };
        if order.status != "pending" {
            return Err(DbErr::Custom("inbound order is not pending".to_owned()));
        }
        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_inbound_orders
                SET status = 'rejected',
                    rejected_by_user_id = ?,
                    rejected_at = ?,
                    updated_at = ?
                WHERE id = ? AND status = 'pending'
                "#,
                vec![
                    user_id.into(),
                    now.clone().into(),
                    now.clone().into(),
                    id.into(),
                ],
            ))
            .await?;
        insert_audit_event_on_connection(
            &transaction,
            user_id,
            "inbound",
            Some(id),
            "rejected",
            Some(r#"{"reason":"rejected_by_user"}"#.to_owned()),
        )
        .await?;
        transaction.commit().await?;

        self.find_inbound_order_by_id(id).await
    }

    /// 创建 pending 出库单和明细；创建阶段不扣减库存。
    pub(crate) async fn create_outbound_order(
        &self,
        input: CreateOutboundOrder,
    ) -> Result<OutboundOrderDetail, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        if input.items.is_empty() {
            return Err(DbErr::Custom(
                "outbound order items must not be empty".to_owned(),
            ));
        }

        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        let result = transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO stock_outbound_orders
                    (destination, status, notes, created_by_user_id, created_at, updated_at)
                VALUES (?, 'pending', ?, ?, ?, ?)
                "#,
                vec![
                    input.destination.clone().into(),
                    input.notes.into(),
                    input.created_by_user_id.into(),
                    now.clone().into(),
                    now.clone().into(),
                ],
            ))
            .await?;
        let order_id = i64::try_from(result.last_insert_id())
            .map_err(|_| DbErr::Custom("outbound order id overflow".to_owned()))?;

        for item in &input.items {
            validate_repository_input(item)?;
            transaction
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    r#"
                    INSERT INTO stock_outbound_order_items
                        (order_id, item_id, quantity, batch_id, location, created_at)
                    VALUES (?, ?, ?, ?, ?, ?)
                    "#,
                    vec![
                        order_id.into(),
                        item.item_id.into(),
                        item.quantity.into(),
                        item.batch_id.into(),
                        item.location.clone().into(),
                        now.clone().into(),
                    ],
                ))
                .await?;
        }
        insert_audit_event_on_connection(
            &transaction,
            input.created_by_user_id,
            "outbound",
            Some(order_id),
            "created",
            Some(format!(
                r#"{{"destination":{},"item_count":{}}}"#,
                json_string(&input.destination),
                input.items.len()
            )),
        )
        .await?;
        transaction.commit().await?;

        self.find_outbound_order_by_id(order_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created outbound order".to_owned()))
    }

    /// 查询出库单详情。
    pub(crate) async fn find_outbound_order_by_id(
        &self,
        id: i64,
    ) -> Result<Option<OutboundOrderDetail>, DbErr> {
        let Some(order) = self.find_outbound_order_record_by_id(id).await? else {
            return Ok(None);
        };
        let items = list_outbound_items_on_connection(self.database, id).await?;

        Ok(Some(OutboundOrderDetail { order, items }))
    }

    /// 分页查询出库单，支持物品和创建时间筛选。
    pub(crate) async fn list_outbound_orders(
        &self,
        input: ListOutboundOrders,
    ) -> Result<Page<OutboundOrderDetail>, DbErr> {
        let limit = input.page_size as i64;
        let offset = ((input.page.saturating_sub(1)) * input.page_size) as i64;
        let total = self.count_outbound_orders(&input).await?;
        let rows = self.query_outbound_orders(&input, limit, offset).await?;
        let mut items = Vec::with_capacity(rows.len());
        for order in rows {
            let order_items = list_outbound_items_on_connection(self.database, order.id).await?;
            items.push(OutboundOrderDetail {
                order,
                items: order_items,
            });
        }

        Ok(Page { items, total })
    }

    /// 审批 pending 出库单；指定批次或 FIFO 扣减、库存流水和审计事件必须在同一事务内完成。
    pub(crate) async fn approve_outbound_order(
        &self,
        id: i64,
        user_id: Option<i64>,
    ) -> Result<Option<OutboundOrderDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        let Some(order) = self.find_outbound_order_record_by_id(id).await? else {
            return Ok(None);
        };
        if order.status != "pending" {
            return Err(DbErr::Custom("outbound order is not pending".to_owned()));
        }
        let order_items = list_outbound_items_on_connection(self.database, id).await?;
        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_outbound_orders
                SET status = 'approved',
                    approved_by_user_id = ?,
                    approved_at = ?,
                    updated_at = ?
                WHERE id = ? AND status = 'pending'
                "#,
                vec![
                    user_id.into(),
                    now.clone().into(),
                    now.clone().into(),
                    id.into(),
                ],
            ))
            .await?;

        for item in &order_items {
            deduct_outbound_item_on_connection(&transaction, item, user_id, &now).await?;
        }
        insert_audit_event_on_connection(
            &transaction,
            user_id,
            "outbound",
            Some(id),
            "approved",
            Some(format!(r#"{{"item_count":{}}}"#, order_items.len())),
        )
        .await?;
        transaction.commit().await?;

        self.find_outbound_order_by_id(id).await
    }

    /// 拒绝 pending 出库单；拒绝不扣减库存。
    pub(crate) async fn reject_outbound_order(
        &self,
        id: i64,
        user_id: Option<i64>,
    ) -> Result<Option<OutboundOrderDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        let Some(order) = self.find_outbound_order_record_by_id(id).await? else {
            return Ok(None);
        };
        if order.status != "pending" {
            return Err(DbErr::Custom("outbound order is not pending".to_owned()));
        }
        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_outbound_orders
                SET status = 'rejected',
                    rejected_by_user_id = ?,
                    rejected_at = ?,
                    updated_at = ?
                WHERE id = ? AND status = 'pending'
                "#,
                vec![
                    user_id.into(),
                    now.clone().into(),
                    now.clone().into(),
                    id.into(),
                ],
            ))
            .await?;
        insert_audit_event_on_connection(
            &transaction,
            user_id,
            "outbound",
            Some(id),
            "rejected",
            Some(r#"{"reason":"rejected_by_user"}"#.to_owned()),
        )
        .await?;
        transaction.commit().await?;

        self.find_outbound_order_by_id(id).await
    }

    /// 查询看板总览；统计只读取当前库存和审批后产生的库存流水。
    pub(crate) async fn dashboard_overview(
        &self,
        slow_moving_days: i64,
    ) -> Result<DashboardOverviewRecord, DbErr> {
        let summary = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT
                    (SELECT COUNT(*) FROM stock_items WHERE deleted_at IS NULL) AS total_items,
                    (
                        SELECT COALESCE(SUM(batches.remaining_quantity), 0.0)
                        FROM stock_batches batches
                        JOIN stock_items items ON items.id = batches.item_id
                        WHERE items.deleted_at IS NULL
                    ) AS total_quantity,
                    (
                        SELECT COALESCE(SUM(batches.remaining_quantity * batches.unit_cost), 0.0)
                        FROM stock_batches batches
                        JOIN stock_items items ON items.id = batches.item_id
                        WHERE items.deleted_at IS NULL
                    ) AS total_value,
                    (
                        SELECT COALESCE(SUM(quantity_delta), 0.0)
                        FROM stock_movements
                        WHERE movement_type = 'inbound'
                          AND created_at >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-3 days')
                    ) AS inbound_3d,
                    (
                        SELECT COALESCE(SUM(-quantity_delta), 0.0)
                        FROM stock_movements
                        WHERE movement_type = 'outbound'
                          AND created_at >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-3 days')
                    ) AS outbound_3d
                "#,
                [],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("dashboard overview".to_owned()))?;
        let slow_moving_items = self.list_slow_moving_items(slow_moving_days).await?;

        Ok(DashboardOverviewRecord {
            total_items: summary.try_get("", "total_items")?,
            total_quantity: summary.try_get("", "total_quantity")?,
            total_value: summary.try_get("", "total_value")?,
            inbound_3d: summary.try_get("", "inbound_3d")?,
            outbound_3d: summary.try_get("", "outbound_3d")?,
            slow_moving_items,
        })
    }

    /// 查询每日出入库趋势；无流水日期也会返回 0，便于前端直接绘图。
    pub(crate) async fn dashboard_trends(
        &self,
        days: i64,
    ) -> Result<Vec<DailyMovementTrendRecord>, DbErr> {
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                WITH RECURSIVE dates(date, remaining) AS (
                    SELECT date('now', ?), ?
                    UNION ALL
                    SELECT date(date, '+1 day'), remaining - 1
                    FROM dates
                    WHERE remaining > 0
                ),
                movement_daily AS (
                    SELECT
                        date(created_at) AS date,
                        SUM(CASE WHEN movement_type = 'inbound' THEN quantity_delta ELSE 0.0 END) AS inbound_quantity,
                        SUM(CASE WHEN movement_type = 'outbound' THEN -quantity_delta ELSE 0.0 END) AS outbound_quantity
                    FROM stock_movements
                    WHERE movement_type IN ('inbound', 'outbound')
                      AND date(created_at) >= date('now', ?)
                    GROUP BY date(created_at)
                )
                SELECT
                    dates.date AS date,
                    COALESCE(movement_daily.inbound_quantity, 0.0) AS inbound_quantity,
                    COALESCE(movement_daily.outbound_quantity, 0.0) AS outbound_quantity
                FROM dates
                LEFT JOIN movement_daily ON movement_daily.date = dates.date
                ORDER BY dates.date ASC
                "#,
                vec![
                    format!("-{} days", days - 1).into(),
                    (days - 1).into(),
                    format!("-{} days", days - 1).into(),
                ],
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
                Ok(DailyMovementTrendRecord {
                    date: row.try_get("", "date")?,
                    inbound_quantity: row.try_get("", "inbound_quantity")?,
                    outbound_quantity: row.try_get("", "outbound_quantity")?,
                })
            })
            .collect()
    }

    /// 整体替换指定物品的替代料列表；替换、环路校验和审计事件必须在同一事务内完成。
    pub(crate) async fn replace_substitutes(
        &self,
        item_id: i64,
        substitutes: Vec<BindStockSubstitute>,
        user_id: Option<i64>,
    ) -> Result<Option<Vec<StockSubstituteRecord>>, DbErr>
    where
        C: TransactionTrait,
    {
        if self.find_active_item_by_id(item_id).await?.is_none() {
            return Ok(None);
        }
        validate_substitute_inputs(item_id, &substitutes)?;
        for substitute in &substitutes {
            if self
                .find_active_item_by_id(substitute.substitute_item_id)
                .await?
                .is_none()
            {
                return Err(DbErr::Custom("substitute item not found".to_owned()));
            }
            if self
                .substitute_would_create_cycle(item_id, substitute.substitute_item_id)
                .await?
            {
                return Err(DbErr::Custom("substitute cycle".to_owned()));
            }
        }

        let transaction = self.database.begin().await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "DELETE FROM stock_substitutes WHERE item_id = ?",
                [item_id.into()],
            ))
            .await?;
        for substitute in substitutes {
            validate_repository_input(&substitute)?;
            transaction
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    r#"
                    INSERT INTO stock_substitutes
                        (item_id, substitute_item_id, priority, notes, created_by_user_id)
                    VALUES (?, ?, ?, ?, ?)
                    "#,
                    vec![
                        item_id.into(),
                        substitute.substitute_item_id.into(),
                        substitute.priority.into(),
                        substitute.notes.into(),
                        user_id.into(),
                    ],
                ))
                .await?;
        }
        insert_audit_event_on_connection(
            &transaction,
            user_id,
            "substitute",
            Some(item_id),
            "linked",
            Some(r#"{"mode":"replace"}"#.to_owned()),
        )
        .await?;
        transaction.commit().await?;

        self.list_substitutes(item_id).await.map(Some)
    }

    /// 查询指定物品的替代料列表；只返回未软删除的主物品和替代物品。
    pub(crate) async fn list_substitutes(
        &self,
        item_id: i64,
    ) -> Result<Vec<StockSubstituteRecord>, DbErr> {
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT
                    substitutes.item_id,
                    substitutes.substitute_item_id,
                    substitute_items.name AS substitute_item_name,
                    COALESCE(SUM(batches.remaining_quantity), 0.0) AS quantity,
                    substitutes.priority,
                    substitutes.notes,
                    substitutes.created_by_user_id,
                    substitutes.created_at
                FROM stock_substitutes substitutes
                JOIN stock_items items
                    ON items.id = substitutes.item_id
                   AND items.deleted_at IS NULL
                JOIN stock_items substitute_items
                    ON substitute_items.id = substitutes.substitute_item_id
                   AND substitute_items.deleted_at IS NULL
                LEFT JOIN stock_batches batches
                    ON batches.item_id = substitute_items.id
                WHERE substitutes.item_id = ?
                GROUP BY
                    substitutes.item_id,
                    substitutes.substitute_item_id,
                    substitute_items.name,
                    substitutes.priority,
                    substitutes.notes,
                    substitutes.created_by_user_id,
                    substitutes.created_at
                ORDER BY substitutes.priority ASC, substitutes.substitute_item_id ASC
                "#,
                [item_id.into()],
            ))
            .await?;

        rows.into_iter().map(substitute_from_row).collect()
    }

    /// 解绑单个替代料关系；返回 false 表示关系原本不存在。
    pub(crate) async fn delete_substitute(
        &self,
        item_id: i64,
        substitute_item_id: i64,
        user_id: Option<i64>,
    ) -> Result<bool, DbErr>
    where
        C: TransactionTrait,
    {
        let transaction = self.database.begin().await?;
        let result = transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                DELETE FROM stock_substitutes
                WHERE item_id = ? AND substitute_item_id = ?
                "#,
                vec![item_id.into(), substitute_item_id.into()],
            ))
            .await?;
        let deleted = result.rows_affected() > 0;
        if deleted {
            insert_audit_event_on_connection(
                &transaction,
                user_id,
                "substitute",
                Some(item_id),
                "unlinked",
                Some(format!(r#"{{"substitute_item_id":{substitute_item_id}}}"#)),
            )
            .await?;
        }
        transaction.commit().await?;

        Ok(deleted)
    }

    /// 分页查询审计事件，支持实体、动作、用户和时间筛选。
    pub(crate) async fn list_audit_events(
        &self,
        input: ListAuditEvents,
    ) -> Result<Page<AuditEventRecord>, DbErr> {
        let limit = input.page_size as i64;
        let offset = ((input.page.saturating_sub(1)) * input.page_size) as i64;
        let total = self.count_audit_events(&input).await?;
        let items = self.query_audit_events(&input, limit, offset).await?;

        Ok(Page { items, total })
    }

    /// 查询未软删除物品详情。
    pub(crate) async fn find_active_item_by_id(
        &self,
        id: i64,
    ) -> Result<Option<stock_item::Model>, DbErr> {
        stock_item::Entity::find_by_id(id)
            .filter(stock_item::Column::DeletedAt.is_null())
            .one(self.database)
            .await
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

    /// 分页查询未软删除物品，支持名称/SKU 模糊搜索和模板筛选。
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

    async fn count_audit_events(&self, input: &ListAuditEvents) -> Result<u64, DbErr> {
        let (where_clause, values) = audit_event_filters(input);
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!("SELECT COUNT(*) AS count FROM audit_events {where_clause}"),
                values,
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("audit event count".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count as u64)
    }

    async fn query_audit_events(
        &self,
        input: &ListAuditEvents,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditEventRecord>, DbErr> {
        let (where_clause, mut values) = audit_event_filters(input);
        values.push(limit.into());
        values.push(offset.into());
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!(
                    r#"
                    SELECT
                        audit_events.id,
                        audit_events.timestamp,
                        audit_events.user_id,
                        auth_users.username,
                        audit_events.entity_type,
                        audit_events.entity_id,
                        audit_events.action,
                        audit_events.details_json
                    FROM audit_events
                    LEFT JOIN auth_users ON auth_users.id = audit_events.user_id
                    {where_clause}
                    ORDER BY audit_events.timestamp DESC, audit_events.id DESC
                    LIMIT ? OFFSET ?
                    "#
                ),
                values,
            ))
            .await?;

        rows.into_iter().map(audit_event_from_row).collect()
    }

    async fn substitute_would_create_cycle(
        &self,
        item_id: i64,
        substitute_item_id: i64,
    ) -> Result<bool, DbErr> {
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                WITH RECURSIVE substitute_path(current_item_id) AS (
                    SELECT ?
                    UNION
                    SELECT substitutes.substitute_item_id
                    FROM stock_substitutes substitutes
                    JOIN substitute_path path
                        ON substitutes.item_id = path.current_item_id
                    JOIN stock_items items
                        ON items.id = substitutes.substitute_item_id
                       AND items.deleted_at IS NULL
                    WHERE substitutes.item_id != ?
                )
                SELECT EXISTS(
                    SELECT 1
                    FROM substitute_path
                    WHERE current_item_id = ?
                ) AS has_cycle
                "#,
                vec![substitute_item_id.into(), item_id.into(), item_id.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("substitute cycle check".to_owned()))?;
        let has_cycle: i64 = row.try_get("", "has_cycle")?;

        Ok(has_cycle != 0)
    }

    async fn list_slow_moving_items(
        &self,
        slow_moving_days: i64,
    ) -> Result<Vec<SlowMovingStockItemRecord>, DbErr> {
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                WITH item_stock AS (
                    SELECT
                        items.id AS item_id,
                        items.name AS item_name,
                        COALESCE(SUM(batches.remaining_quantity), 0.0) AS quantity,
                        COALESCE(SUM(batches.remaining_quantity * batches.unit_cost), 0.0) AS value,
                        MAX(movements.created_at) AS last_movement_at
                    FROM stock_items items
                    LEFT JOIN stock_batches batches ON batches.item_id = items.id
                    LEFT JOIN stock_movements movements ON movements.item_id = items.id
                    WHERE items.deleted_at IS NULL
                    GROUP BY items.id, items.name
                )
                SELECT
                    item_id,
                    item_name,
                    quantity,
                    value,
                    CAST(COALESCE(julianday('now') - julianday(last_movement_at), ? + 1) AS INTEGER)
                        AS days_since_last_movement
                FROM item_stock
                WHERE quantity > 0
                  AND COALESCE(julianday('now') - julianday(last_movement_at), ? + 1) >= ?
                ORDER BY days_since_last_movement DESC, item_id ASC
                "#,
                vec![
                    slow_moving_days.into(),
                    slow_moving_days.into(),
                    slow_moving_days.into(),
                ],
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
                Ok(SlowMovingStockItemRecord {
                    item_id: row.try_get("", "item_id")?,
                    item_name: row.try_get("", "item_name")?,
                    quantity: row.try_get("", "quantity")?,
                    value: row.try_get("", "value")?,
                    days_since_last_movement: row.try_get("", "days_since_last_movement")?,
                })
            })
            .collect()
    }

    async fn find_inbound_order_record_by_id(
        &self,
        id: i64,
    ) -> Result<Option<InboundOrderRecord>, DbErr> {
        self.database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                inbound_order_select_sql("WHERE id = ?"),
                [id.into()],
            ))
            .await?
            .map(inbound_order_from_row)
            .transpose()
    }

    async fn count_inbound_orders(&self, input: &ListInboundOrders) -> Result<u64, DbErr> {
        let (where_clause, values) = inbound_order_filters(input);
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!("SELECT COUNT(*) AS count FROM stock_inbound_orders {where_clause}"),
                values,
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("inbound order count".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count as u64)
    }

    async fn query_inbound_orders(
        &self,
        input: &ListInboundOrders,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<InboundOrderRecord>, DbErr> {
        let (where_clause, mut values) = inbound_order_filters(input);
        values.push(limit.into());
        values.push(offset.into());
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!(
                    "{} {where_clause} ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?",
                    inbound_order_select_sql("")
                ),
                values,
            ))
            .await?;

        rows.into_iter().map(inbound_order_from_row).collect()
    }

    async fn find_outbound_order_record_by_id(
        &self,
        id: i64,
    ) -> Result<Option<OutboundOrderRecord>, DbErr> {
        self.database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                outbound_order_select_sql("WHERE id = ?"),
                [id.into()],
            ))
            .await?
            .map(outbound_order_from_row)
            .transpose()
    }

    async fn count_outbound_orders(&self, input: &ListOutboundOrders) -> Result<u64, DbErr> {
        let (where_clause, values) = outbound_order_filters(input);
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!("SELECT COUNT(*) AS count FROM stock_outbound_orders {where_clause}"),
                values,
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("outbound order count".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count as u64)
    }

    async fn query_outbound_orders(
        &self,
        input: &ListOutboundOrders,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OutboundOrderRecord>, DbErr> {
        let (where_clause, mut values) = outbound_order_filters(input);
        values.push(limit.into());
        values.push(offset.into());
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!(
                    "{} {where_clause} ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?",
                    outbound_order_select_sql("")
                ),
                values,
            ))
            .await?;

        rows.into_iter().map(outbound_order_from_row).collect()
    }
}

fn inbound_order_filters(input: &ListInboundOrders) -> (String, Vec<Value>) {
    let mut clauses = Vec::new();
    let mut values = Vec::new();

    if let Some(item_id) = input.item_id {
        clauses.push(
            "EXISTS (SELECT 1 FROM stock_inbound_order_items items WHERE items.order_id = stock_inbound_orders.id AND items.item_id = ?)",
        );
        values.push(item_id.into());
    }
    if let Some(date_from) = input.date_from.as_ref() {
        clauses.push("created_at >= ?");
        values.push(date_from.clone().into());
    }
    if let Some(date_to) = input.date_to.as_ref() {
        clauses.push("created_at <= ?");
        values.push(date_to.clone().into());
    }

    if clauses.is_empty() {
        (String::new(), values)
    } else {
        (format!("WHERE {}", clauses.join(" AND ")), values)
    }
}

fn inbound_order_select_sql(where_clause: &str) -> String {
    format!(
        r#"
        SELECT id, source, status, notes, created_by_user_id, approved_by_user_id,
               rejected_by_user_id, created_at, updated_at, approved_at, rejected_at
        FROM stock_inbound_orders
        {where_clause}
        "#
    )
}

fn inbound_order_from_row(row: sea_orm::QueryResult) -> Result<InboundOrderRecord, DbErr> {
    Ok(InboundOrderRecord {
        id: row.try_get("", "id")?,
        source: row.try_get("", "source")?,
        status: row.try_get("", "status")?,
        notes: row.try_get("", "notes")?,
        created_by_user_id: row.try_get("", "created_by_user_id")?,
        approved_by_user_id: row.try_get("", "approved_by_user_id")?,
        rejected_by_user_id: row.try_get("", "rejected_by_user_id")?,
        created_at: row.try_get("", "created_at")?,
        updated_at: row.try_get("", "updated_at")?,
        approved_at: row.try_get("", "approved_at")?,
        rejected_at: row.try_get("", "rejected_at")?,
    })
}

fn outbound_order_filters(input: &ListOutboundOrders) -> (String, Vec<Value>) {
    let mut clauses = Vec::new();
    let mut values = Vec::new();

    if let Some(item_id) = input.item_id {
        clauses.push(
            "EXISTS (SELECT 1 FROM stock_outbound_order_items items WHERE items.order_id = stock_outbound_orders.id AND items.item_id = ?)",
        );
        values.push(item_id.into());
    }
    if let Some(date_from) = input.date_from.as_ref() {
        clauses.push("created_at >= ?");
        values.push(date_from.clone().into());
    }
    if let Some(date_to) = input.date_to.as_ref() {
        clauses.push("created_at <= ?");
        values.push(date_to.clone().into());
    }

    if clauses.is_empty() {
        (String::new(), values)
    } else {
        (format!("WHERE {}", clauses.join(" AND ")), values)
    }
}

fn outbound_order_select_sql(where_clause: &str) -> String {
    format!(
        r#"
        SELECT id, destination, status, notes, created_by_user_id, approved_by_user_id,
               rejected_by_user_id, created_at, updated_at, approved_at, rejected_at
        FROM stock_outbound_orders
        {where_clause}
        "#
    )
}

fn outbound_order_from_row(row: sea_orm::QueryResult) -> Result<OutboundOrderRecord, DbErr> {
    Ok(OutboundOrderRecord {
        id: row.try_get("", "id")?,
        destination: row.try_get("", "destination")?,
        status: row.try_get("", "status")?,
        notes: row.try_get("", "notes")?,
        created_by_user_id: row.try_get("", "created_by_user_id")?,
        approved_by_user_id: row.try_get("", "approved_by_user_id")?,
        rejected_by_user_id: row.try_get("", "rejected_by_user_id")?,
        created_at: row.try_get("", "created_at")?,
        updated_at: row.try_get("", "updated_at")?,
        approved_at: row.try_get("", "approved_at")?,
        rejected_at: row.try_get("", "rejected_at")?,
    })
}

fn audit_event_filters(input: &ListAuditEvents) -> (String, Vec<Value>) {
    let mut clauses = Vec::new();
    let mut values = Vec::new();

    if let Some(entity_type) = input.entity_type.as_ref() {
        clauses.push("entity_type = ?");
        values.push(entity_type.clone().into());
    }
    if let Some(entity_id) = input.entity_id {
        clauses.push("entity_id = ?");
        values.push(entity_id.into());
    }
    if let Some(action) = input.action.as_ref() {
        clauses.push("action = ?");
        values.push(action.clone().into());
    }
    if let Some(user_id) = input.user_id {
        clauses.push("user_id = ?");
        values.push(user_id.into());
    }
    if let Some(date_from) = input.date_from.as_ref() {
        clauses.push("timestamp >= ?");
        values.push(date_from.clone().into());
    }
    if let Some(date_to) = input.date_to.as_ref() {
        clauses.push("timestamp <= ?");
        values.push(date_to.clone().into());
    }

    if clauses.is_empty() {
        (String::new(), values)
    } else {
        (format!("WHERE {}", clauses.join(" AND ")), values)
    }
}

fn audit_event_from_row(row: sea_orm::QueryResult) -> Result<AuditEventRecord, DbErr> {
    Ok(AuditEventRecord {
        id: row.try_get("", "id")?,
        timestamp: row.try_get("", "timestamp")?,
        user_id: row.try_get("", "user_id")?,
        username: row.try_get("", "username")?,
        entity_type: row.try_get("", "entity_type")?,
        entity_id: row.try_get("", "entity_id")?,
        action: row.try_get("", "action")?,
        details_json: row.try_get("", "details_json")?,
    })
}

async fn list_inbound_items_on_connection<C>(
    connection: &C,
    order_id: i64,
) -> Result<Vec<InboundOrderItemRecord>, DbErr>
where
    C: ConnectionTrait,
{
    let rows = connection
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT id, order_id, item_id, quantity, unit_price, location, batch_no,
                   expires_at, ext_attributes_json, created_at
            FROM stock_inbound_order_items
            WHERE order_id = ?
            ORDER BY id ASC
            "#,
            [order_id.into()],
        ))
        .await?;

    rows.into_iter()
        .map(|row| {
            Ok(InboundOrderItemRecord {
                id: row.try_get("", "id")?,
                order_id: row.try_get("", "order_id")?,
                item_id: row.try_get("", "item_id")?,
                quantity: row.try_get("", "quantity")?,
                unit_price: row.try_get("", "unit_price")?,
                location: row.try_get("", "location")?,
                batch_no: row.try_get("", "batch_no")?,
                expires_at: row.try_get("", "expires_at")?,
                ext_attributes_json: row.try_get("", "ext_attributes_json")?,
                created_at: row.try_get("", "created_at")?,
            })
        })
        .collect()
}

async fn list_outbound_items_on_connection<C>(
    connection: &C,
    order_id: i64,
) -> Result<Vec<OutboundOrderItemRecord>, DbErr>
where
    C: ConnectionTrait,
{
    let rows = connection
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT id, order_id, item_id, quantity, batch_id, location, created_at
            FROM stock_outbound_order_items
            WHERE order_id = ?
            ORDER BY id ASC
            "#,
            [order_id.into()],
        ))
        .await?;

    rows.into_iter()
        .map(|row| {
            Ok(OutboundOrderItemRecord {
                id: row.try_get("", "id")?,
                order_id: row.try_get("", "order_id")?,
                item_id: row.try_get("", "item_id")?,
                quantity: row.try_get("", "quantity")?,
                batch_id: row.try_get("", "batch_id")?,
                location: row.try_get("", "location")?,
                created_at: row.try_get("", "created_at")?,
            })
        })
        .collect()
}

async fn deduct_outbound_item_on_connection<C>(
    connection: &C,
    item: &OutboundOrderItemRecord,
    user_id: Option<i64>,
    now: &str,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let batches = if let Some(batch_id) = item.batch_id {
        vec![
            find_batch_for_deduction_on_connection(connection, item.item_id, batch_id)
                .await?
                .ok_or_else(|| DbErr::Custom("insufficient stock".to_owned()))?,
        ]
    } else {
        list_fifo_batches_for_deduction_on_connection(connection, item.item_id).await?
    };

    let mut remaining_to_deduct = item.quantity;
    for batch in batches {
        if remaining_to_deduct <= 0.0 {
            break;
        }
        let deducted = remaining_to_deduct.min(batch.remaining_quantity);
        let new_remaining = batch.remaining_quantity - deducted;
        connection
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_batches
                SET remaining_quantity = ?, updated_at = ?
                WHERE id = ? AND remaining_quantity = ?
                "#,
                vec![
                    new_remaining.into(),
                    now.to_owned().into(),
                    batch.id.into(),
                    batch.remaining_quantity.into(),
                ],
            ))
            .await?;
        remaining_to_deduct -= deducted;
        let balance_after = current_item_quantity_on_connection(connection, item.item_id).await?;
        connection
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO stock_movements
                    (item_id, batch_id, movement_type, quantity_delta, unit_cost, balance_after, outbound_order_item_id, created_by_user_id, created_at)
                VALUES (?, ?, 'outbound', ?, ?, ?, ?, ?, ?)
                "#,
                vec![
                    item.item_id.into(),
                    batch.id.into(),
                    (-deducted).into(),
                    batch.unit_cost.into(),
                    balance_after.into(),
                    item.id.into(),
                    user_id.into(),
                    now.to_owned().into(),
                ],
            ))
            .await?;
    }

    if remaining_to_deduct > 0.000_000_1 {
        Err(DbErr::Custom("insufficient stock".to_owned()))
    } else {
        Ok(())
    }
}

async fn find_batch_for_deduction_on_connection<C>(
    connection: &C,
    item_id: i64,
    batch_id: i64,
) -> Result<Option<StockBatchForDeduction>, DbErr>
where
    C: ConnectionTrait,
{
    connection
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT id, remaining_quantity, unit_cost
            FROM stock_batches
            WHERE id = ? AND item_id = ? AND remaining_quantity > 0
            "#,
            [batch_id.into(), item_id.into()],
        ))
        .await?
        .map(batch_for_deduction_from_row)
        .transpose()
}

async fn list_fifo_batches_for_deduction_on_connection<C>(
    connection: &C,
    item_id: i64,
) -> Result<Vec<StockBatchForDeduction>, DbErr>
where
    C: ConnectionTrait,
{
    let rows = connection
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT id, remaining_quantity, unit_cost
            FROM stock_batches
            WHERE item_id = ? AND remaining_quantity > 0
            ORDER BY expires_at IS NULL ASC, expires_at ASC, received_at ASC, id ASC
            "#,
            [item_id.into()],
        ))
        .await?;

    rows.into_iter().map(batch_for_deduction_from_row).collect()
}

fn batch_for_deduction_from_row(
    row: sea_orm::QueryResult,
) -> Result<StockBatchForDeduction, DbErr> {
    Ok(StockBatchForDeduction {
        id: row.try_get("", "id")?,
        remaining_quantity: row.try_get("", "remaining_quantity")?,
        unit_cost: row.try_get("", "unit_cost")?,
    })
}

fn validate_substitute_inputs(
    item_id: i64,
    substitutes: &[BindStockSubstitute],
) -> Result<(), DbErr> {
    let mut ids = HashSet::with_capacity(substitutes.len());
    let mut priorities = HashSet::with_capacity(substitutes.len());
    for substitute in substitutes {
        validate_repository_input(substitute)?;
        if substitute.substitute_item_id == item_id {
            return Err(DbErr::Custom("substitute self reference".to_owned()));
        }
        if !ids.insert(substitute.substitute_item_id) {
            return Err(DbErr::Custom("duplicate substitute item".to_owned()));
        }
        if !priorities.insert(substitute.priority) {
            return Err(DbErr::Custom("duplicate substitute priority".to_owned()));
        }
    }

    Ok(())
}

fn substitute_from_row(row: sea_orm::QueryResult) -> Result<StockSubstituteRecord, DbErr> {
    Ok(StockSubstituteRecord {
        item_id: row.try_get("", "item_id")?,
        substitute_item_id: row.try_get("", "substitute_item_id")?,
        substitute_item_name: row.try_get("", "substitute_item_name")?,
        quantity: row.try_get("", "quantity")?,
        priority: row.try_get("", "priority")?,
        notes: row.try_get("", "notes")?,
        created_by_user_id: row.try_get("", "created_by_user_id")?,
        created_at: row.try_get("", "created_at")?,
    })
}

async fn current_item_quantity_on_connection<C>(connection: &C, item_id: i64) -> Result<f64, DbErr>
where
    C: ConnectionTrait,
{
    let row = connection
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT COALESCE(SUM(remaining_quantity), 0.0) AS quantity
            FROM stock_batches
            WHERE item_id = ?
            "#,
            [item_id.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("stock batch balance".to_owned()))?;

    row.try_get("", "quantity")
}

async fn insert_audit_event_on_connection<C>(
    connection: &C,
    user_id: Option<i64>,
    entity_type: &str,
    entity_id: Option<i64>,
    action: &str,
    details_json: Option<String>,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    connection
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO audit_events (user_id, entity_type, entity_id, action, details_json)
            VALUES (?, ?, ?, ?, ?)
            "#,
            vec![
                user_id.into(),
                entity_type.into(),
                entity_id.into(),
                action.into(),
                details_json.into(),
            ],
        ))
        .await?;

    Ok(())
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
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

fn validate_positive_f64(value: &f64, _: &()) -> garde::Result {
    if value.is_finite() && *value > 0.0 {
        Ok(())
    } else {
        Err(garde::Error::new("must_be_positive"))
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
        sql.push_str(" AND (lower(name) LIKE ? OR lower(sku) LIKE ?)");
        values.push(search_like.into());
        values.push(search_like.into());
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
