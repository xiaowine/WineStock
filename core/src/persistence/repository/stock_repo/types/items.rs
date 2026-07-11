//! 物品、分类和物品属性仓储模型。
//!
//! 本模块属于 `core` 持久化层，只描述物品子域的仓储边界数据。

use crate::{
    persistence::entity::item_attribute,
    validation::{validate_json_text, validate_not_blank, validate_optional_not_blank},
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
    /// 物品分类 ID；为空表示暂不分类。
    #[garde(skip)]
    pub category_id: Option<i64>,
    /// 可选物品属性模板 ID；模板只提供预设字段。
    #[garde(skip)]
    pub attribute_template_id: Option<i64>,
    /// 必选物品主图文件对象 ID。
    #[garde(range(min = 1))]
    pub image_file_id: i64,
    /// 上传主图的所有者用户 ID；仓储事务据此完成最终占用检查。
    #[garde(range(min = 1))]
    pub image_owner_user_id: i64,
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
    /// 物品固有属性，允许包含模板字段和自定义字段。
    #[garde(dive)]
    pub attributes: Vec<ItemAttributeInput>,
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
    /// 分类 ID；外层表示是否修改，内层表示是否清空。
    #[garde(skip)]
    pub category_id: Option<Option<i64>>,
    /// 属性模板 ID；外层表示是否修改，内层表示是否清空。
    #[garde(skip)]
    pub attribute_template_id: Option<Option<i64>>,
    /// 新物品主图文件对象 ID；为空表示保留当前图片。
    #[garde(range(min = 1))]
    pub image_file_id: Option<i64>,
    /// 新主图上传者 ID；更换图片时必须存在。
    #[garde(custom(crate::persistence::repository::validation::validate_optional_positive_id))]
    pub image_owner_user_id: Option<i64>,
    /// 计量单位，存在时裁剪后不能为空。
    #[garde(length(min = 1, max = 32), custom(validate_optional_not_blank))]
    pub unit: Option<String>,
    /// 物品描述；外层表示是否修改，内层表示是否清空。
    #[garde(skip)]
    pub description: Option<Option<String>>,
    /// 参考单价；外层表示是否修改，内层表示是否清空。
    #[garde(skip)]
    pub default_price: Option<Option<f64>>,
    /// 再订货点；外层表示是否修改，内层表示是否清空。
    #[garde(skip)]
    pub reorder_point: Option<Option<f64>>,
    /// 物品固有属性；存在时整体替换。
    #[garde(skip)]
    pub attributes: Option<Vec<ItemAttributeInput>>,
}

/// 库存物品分页查询条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListStockItems {
    /// 页码，从 1 开始。
    pub page: u64,
    /// 每页数量，服务层负责限制最大值。
    pub page_size: u64,
    /// 物品基础资料和实际属性模糊搜索关键字。
    pub search: Option<String>,
    /// 按分类 ID 筛选。
    pub category_id: Option<i64>,
}

/// 创建物品分类的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct CreateItemCategory {
    /// 分类名称，未软删除记录内唯一。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,
    /// 分类说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,
    /// 分类展示顺序。
    #[garde(range(min = 0))]
    pub sort_order: i32,
}

/// 更新物品分类的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct UpdateItemCategory {
    /// 分类名称；为空表示不修改。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,
    /// 分类说明；外层表示是否修改，内层表示是否清空。
    #[garde(skip)]
    pub description: Option<Option<String>>,
    /// 分类展示顺序；为空表示不修改。
    #[garde(range(min = 0))]
    pub sort_order: Option<i32>,
}

/// 创建或整体替换物品属性时使用的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct ItemAttributeInput {
    /// 可选模板字段来源；自定义属性为空。
    #[garde(skip)]
    pub template_field_id: Option<i64>,
    /// 属性名称，同一物品内唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub field_name: String,
    /// 稳定属性类型代码。
    #[garde(length(min = 1, max = 32), custom(validate_not_blank))]
    pub field_type: String,
    /// JSON 编码后的属性值。
    #[garde(
        length(min = 1, max = 8192),
        custom(validate_not_blank),
        custom(validate_json_text)
    )]
    pub value_json: String,
    /// 可选计量单位。
    #[garde(length(min = 1, max = 32), custom(validate_optional_not_blank))]
    pub unit: Option<String>,
    /// 属性展示顺序。
    #[garde(range(min = 0))]
    pub sort_order: i32,
    /// file 属性引用的临时上传文件 ID。
    #[garde(skip)]
    pub file_object_id: Option<i64>,
    /// 上传文件所有者 ID；绑定 file 属性时必须存在。
    #[garde(skip)]
    pub file_owner_user_id: Option<i64>,
}

/// 物品属性及其稳定数据库身份。
pub(crate) type ItemAttributeRecord = item_attribute::Model;

/// 物品列表读取模型，包含基础资料和固有属性。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StockItemListRecord {
    /// 物品基础资料。
    pub item: crate::persistence::entity::stock_item::Model,
    /// 物品固有属性。
    pub attributes: Vec<ItemAttributeRecord>,
}

/// 库存物品详情读取模型，包含基础资料和当前有效批次聚合。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StockItemDetail {
    /// 物品基础资料。
    pub item: crate::persistence::entity::stock_item::Model,
    /// 当前剩余库存总量。
    pub current_quantity: f64,
    /// 当前库存价值。
    pub inventory_value: f64,
    /// 当前库存按库位聚合后的分布。
    pub locations: Vec<StockItemLocationRecord>,
    /// 当前仍有余额的批次摘要。
    pub batches: Vec<StockItemBatchRecord>,
    /// 物品固有属性，按展示顺序返回。
    pub attributes: Vec<ItemAttributeRecord>,
}

/// 物品当前库存库位聚合读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StockItemLocationRecord {
    /// 库位 ID。
    pub location_id: i64,
    /// 库位编码。
    pub location_code: String,
    /// 库位名称。
    pub location_name: String,
    /// 该库位当前剩余库存量。
    pub quantity: f64,
    /// 该库位当前库存价值。
    pub value: f64,
    /// 该库位当前仍有余额的批次数。
    pub batch_count: i64,
}

/// 物品当前库存批次摘要读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StockItemBatchRecord {
    /// 批次 ID。
    pub id: i64,
    /// 批次号。
    pub batch_no: String,
    /// 批次库位 ID。
    pub location_id: i64,
    /// 批次库位编码。
    pub location_code: String,
    /// 批次库位名称。
    pub location_name: String,
    /// 入库时的初始数量。
    pub initial_quantity: f64,
    /// 当前剩余数量。
    pub remaining_quantity: f64,
    /// 批次单价。
    pub unit_cost: f64,
    /// 当前批次库存价值。
    pub value: f64,
    /// 入库审批时间。
    pub received_at: String,
    /// 有效期。
    pub expires_at: Option<String>,
}
