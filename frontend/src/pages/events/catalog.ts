// 本文件拥有审计实体、动作和详情字段的显示字典；未知值必须回退原始代码。

export const CUSTOM_EVENT_FILTER = "__custom__";

export interface EventCatalogOption {
  value: string;
  label: string;
}

export const eventEntityOptions: readonly EventCatalogOption[] = [
  { value: "item", label: "物品" },
  { value: "item_category", label: "物品分类" },
  { value: "item_attribute_template", label: "物品属性模板" },
  { value: "user", label: "用户" },
  { value: "inbound", label: "入库单" },
  { value: "outbound", label: "出库单" },
  { value: "location_group", label: "库位分组" },
  { value: "location", label: "库位" },
  { value: "location_transfer", label: "移库记录" },
  { value: "substitute", label: "替代关系" },
];

export const eventActionOptions: readonly EventCatalogOption[] = [
  { value: "created", label: "创建" },
  { value: "updated", label: "更新" },
  { value: "deleted", label: "删除" },
  { value: "approved", label: "审批通过" },
  { value: "rejected", label: "驳回" },
  { value: "linked", label: "建立关联" },
  { value: "unlinked", label: "解除关联" },
  { value: "moved", label: "移动" },
];

const entityLabels = new Map(eventEntityOptions.map((option) => [option.value, option.label]));
const actionLabels = new Map(eventActionOptions.map((option) => [option.value, option.label]));

const fieldLabels: Readonly<Record<string, string>> = {
  added_substitute_item_ids: "新增替代物品",
  attribute_template_id: "物品属性模板",
  batch_id: "批次",
  category_id: "物品分类",
  changed_fields: "变更字段",
  default_price: "默认价格",
  description: "说明",
  destination: "出库去向",
  field: "变更项目",
  field_count: "字段数量",
  first_user: "首个用户",
  from_location_id: "原库位",
  group_id: "所属分组",
  image_file_id: "主图文件",
  item_count: "明细数量",
  item_id: "物品",
  mode: "操作模式",
  name: "名称",
  new_permissions: "修改后权限",
  new_status: "修改后状态",
  new_substitute_item_ids: "修改后替代物品",
  notes: "备注",
  parent_id: "上级分组",
  previous_permissions: "修改前权限",
  previous_status: "修改前状态",
  previous_substitute_item_ids: "修改前替代物品",
  quantity: "数量",
  reason: "原因",
  removed_substitute_item_ids: "移除替代物品",
  reorder_point: "补货点",
  sku: "编号",
  sort_order: "排序",
  source: "入库来源",
  source_template_id: "来源模板",
  substitute_item_id: "替代物品",
  to_location_id: "目标库位",
  unit: "单位",
  username: "用户名",
};

/** 已知实体返回中文，未知实体保留原始代码。 */
export function eventEntityLabel(value: string): string {
  return entityLabels.get(value) ?? value;
}

/** 已知动作返回中文，未知动作保留原始代码。 */
export function eventActionLabel(value: string): string {
  return actionLabels.get(value) ?? value;
}

/** 字段键返回稳定中文；未知键保留原始值。 */
export function eventFieldLabel(value: string): string {
  return fieldLabels[value] ?? value;
}

/** 动作标签的视觉语义。 */
export function eventActionTone(
  value: string,
): "positive" | "accent" | "warn" | "danger" | "neutral" {
  if (["created", "approved", "linked"].includes(value)) return "positive";
  if (["updated", "moved"].includes(value)) return "accent";
  if (value === "unlinked") return "warn";
  if (["deleted", "rejected"].includes(value)) return "danger";
  return "neutral";
}

export function isKnownEntityType(value: string): boolean {
  return entityLabels.has(value);
}

export function isKnownAction(value: string): boolean {
  return actionLabels.has(value);
}
