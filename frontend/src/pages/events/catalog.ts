// 本文件拥有审计实体、动作和详情字段的显示字典；未知值必须回退原始代码。
import { translateMessageOrNull } from "../../i18n";
import type { MessageKeyPath } from "../../i18n/schema";

export const CUSTOM_EVENT_FILTER = "__custom__";

export interface EventCatalogOption {
  value: string;
  /** i18n 消息键（events.entity* / events.action*）。 */
  label: MessageKeyPath;
}

export const eventEntityOptions: readonly EventCatalogOption[] = [
  { value: "item", label: "events.entityItem" },
  { value: "item_category", label: "events.entityItemCategory" },
  { value: "item_attribute_template", label: "events.entityItemAttributeTemplate" },
  { value: "user", label: "events.entityUser" },
  { value: "inbound", label: "events.entityInbound" },
  { value: "outbound", label: "events.entityOutbound" },
  { value: "location_group", label: "events.entityLocationGroup" },
  { value: "location", label: "events.entityLocation" },
  { value: "location_transfer", label: "events.entityLocationTransfer" },
  { value: "substitute", label: "events.entitySubstitute" },
];

export const eventActionOptions: readonly EventCatalogOption[] = [
  { value: "created", label: "events.actionCreated" },
  { value: "updated", label: "events.actionUpdated" },
  { value: "deleted", label: "events.actionDeleted" },
  { value: "approved", label: "events.actionApproved" },
  { value: "rejected", label: "events.actionRejected" },
  { value: "linked", label: "events.actionLinked" },
  { value: "unlinked", label: "events.actionUnlinked" },
  { value: "moved", label: "events.actionMoved" },
];

const entityLabelKeys = new Map(eventEntityOptions.map((option) => [option.value, option.label]));
const actionLabelKeys = new Map(eventActionOptions.map((option) => [option.value, option.label]));

const fieldLabelKeys: Readonly<Record<string, MessageKeyPath>> = {
  added_substitute_item_ids: "events.fieldAddedSubstituteItemIds",
  attribute_template_id: "events.fieldAttributeTemplateId",
  batch_id: "events.fieldBatchId",
  category_id: "events.fieldCategoryId",
  changed_fields: "events.fieldChangedFields",
  default_price: "events.fieldDefaultPrice",
  description: "events.fieldDescription",
  destination: "events.fieldDestination",
  field: "events.fieldChangedField",
  field_count: "events.fieldCount",
  first_user: "events.fieldFirstUser",
  from_location_id: "events.fieldFromLocationId",
  group_id: "events.fieldGroupId",
  image_file_id: "events.fieldImageFileId",
  item_count: "events.fieldItemCount",
  item_id: "events.fieldItemId",
  mode: "events.fieldMode",
  name: "events.fieldName",
  new_permissions: "events.fieldNewPermissions",
  new_status: "events.fieldNewStatus",
  new_substitute_item_ids: "events.fieldNewSubstituteItemIds",
  notes: "events.fieldNotes",
  parent_id: "events.fieldParentId",
  previous_permissions: "events.fieldPreviousPermissions",
  previous_status: "events.fieldPreviousStatus",
  previous_substitute_item_ids: "events.fieldPreviousSubstituteItemIds",
  quantity: "events.fieldQuantity",
  reason: "events.fieldReason",
  removed_substitute_item_ids: "events.fieldRemovedSubstituteItemIds",
  reorder_point: "events.fieldReorderPoint",
  sku: "events.fieldSku",
  sort_order: "events.fieldSortOrder",
  source: "events.fieldSource",
  source_template_id: "events.fieldSourceTemplateId",
  substitute_item_id: "events.fieldSubstituteItemId",
  to_location_id: "events.fieldToLocationId",
  unit: "events.fieldUnit",
  username: "events.fieldUsername",
};

/** 已知实体返回翻译文案，未知实体保留原始代码。 */
export function eventEntityLabel(value: string): string {
  const key = entityLabelKeys.get(value);
  return key ? translateMessageOrNull(key) ?? value : value;
}

/** 已知动作返回翻译文案，未知动作保留原始代码。 */
export function eventActionLabel(value: string): string {
  const key = actionLabelKeys.get(value);
  return key ? translateMessageOrNull(key) ?? value : value;
}

/** 字段键返回翻译文案；未知键保留原始值。 */
export function eventFieldLabel(value: string): string {
  const key = fieldLabelKeys[value];
  return key ? translateMessageOrNull(key) ?? value : value;
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
  return entityLabelKeys.has(value);
}

export function isKnownAction(value: string): boolean {
  return actionLabelKeys.has(value);
}
