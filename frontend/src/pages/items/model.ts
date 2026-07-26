// 本文件拥有可跨页面复用的物品编辑草稿、请求转换与变更快照；它不发起 API 请求。
import type { FileAttributeReference } from "../../api/inbound";
import type { ApiError } from "../../api/errors";
import type {
  ItemAttributeRequest,
  ItemCreateRequest,
  ItemEditorResponse,
  ItemUpdateRequest,
  LcscItemLookupResponse,
} from "../../api/items";
import type {
  ItemAttributeTemplateFieldResponse,
  ItemAttributeTemplateResponse,
} from "../../api/itemAttributeTemplates";
import type { TemplateFieldResponse, TemplateFieldType } from "../../api/templateFields";
import type { FileDraftValue } from "../inbound-draft/model";

export interface ItemAttributeDraft {
  key: string;
  definitionId: number | null;
  custom: boolean;
  fieldName: string;
  fieldType: TemplateFieldType;
  options: string[];
  unitMode: "none" | "fixed" | "select";
  fixedUnit: string;
  unitOptions: string[];
  value: string | number | boolean | FileDraftValue | undefined;
  unit: string;
  fileTemporary: boolean;
}

export interface ItemDraft {
  id: number | null;
  name: string;
  sku: string;
  categoryId: number | null;
  attributeTemplateId: number | null;
  /** 当前物品必选主图草稿。 */
  image: FileDraftValue | null;
  /** 当前主图是否为尚未绑定的临时图片。 */
  imageTemporary: boolean;
  /** 更新成功后需要删除的旧主图文件 ID。 */
  obsoleteImageFileId: number | null;
  unit: string;
  description: string;
  defaultPrice: number | null;
  reorderPoint: number | null;
  attributes: ItemAttributeDraft[];
}

export function emptyItemDraft(): ItemDraft {
  return {
    id: null,
    name: "",
    sku: "",
    categoryId: null,
    attributeTemplateId: null,
    image: null,
    imageTemporary: true,
    obsoleteImageFileId: null,
    unit: "个",
    description: "",
    defaultPrice: null,
    reorderPoint: null,
    attributes: [],
  };
}

/** 将用户确认的立创候选资料一次性覆盖到当前新建草稿，不改变本地分类、模板或库存设置。 */
export function applyLcscLookupToDraft(
  draft: ItemDraft,
  lookup: LcscItemLookupResponse,
  template: ItemAttributeTemplateResponse | null,
): void {
  applyLookupTemplate(draft, template);
  draft.sku = lookup.product_code;
  if (lookup.name.trim()) draft.name = lookup.name.trim();
  if (lookup.description?.trim()) draft.description = lookup.description.trim();
  if (lookup.default_price !== null && lookup.default_price > 0) {
    draft.defaultPrice = lookup.default_price;
  }

  upsertLookupAttribute(draft, "型号", "text", lookup.manufacturer_part);
  upsertLookupAttribute(draft, "品牌", "text", lookup.manufacturer);
  upsertLookupAttribute(draft, "封装", "text", lookup.footprint);
  upsertLookupAttribute(draft, "数据手册", "url", lookup.datasheet_url);
  upsertLookupAttribute(
    draft,
    "参数",
    "text",
    lookup.parameters.map((parameter) => `${parameter.name}：${parameter.value}`).join("\n"),
  );
}

function applyLookupTemplate(
  draft: ItemDraft,
  template: ItemAttributeTemplateResponse | null,
): void {
  if (!template) {
    applyAttributeTemplate(draft, null);
    return;
  }

  const templateNames = new Set(template.fields.map((field) => field.field_name.toLowerCase()));
  const carriedValues = new Map(
    draft.attributes
      .filter(
        (attribute) =>
          attribute.custom && templateNames.has(attribute.fieldName.trim().toLowerCase()),
      )
      .map((attribute) => [
        attribute.fieldName.trim().toLowerCase(),
        { value: attribute.value, unit: attribute.unit },
      ]),
  );
  draft.attributes = draft.attributes.filter(
    (attribute) =>
      !attribute.custom || !templateNames.has(attribute.fieldName.trim().toLowerCase()),
  );
  applyAttributeTemplate(draft, template);
  for (const attribute of draft.attributes.filter((attribute) => !attribute.custom)) {
    const carried = carriedValues.get(attribute.fieldName.trim().toLowerCase());
    if (!carried) continue;
    attribute.value = carried.value;
    attribute.unit = carried.unit;
  }
}

function upsertLookupAttribute(
  draft: ItemDraft,
  fieldName: string,
  fieldType: "text" | "url",
  candidate: string | null,
): void {
  const value = candidate?.trim();
  if (!value) return;
  const normalizedName = fieldName.toLocaleLowerCase();
  let attribute = draft.attributes.find(
    (current) => current.fieldName.trim().toLocaleLowerCase() === normalizedName,
  );

  if (!attribute) {
    attribute = newCustomAttribute();
    attribute.fieldName = fieldName;
    attribute.fieldType = fieldType;
    draft.attributes.push(attribute);
  } else if (attribute.custom && attribute.fieldType !== fieldType) {
    attribute.fieldType = fieldType;
    attribute.options = [];
    attribute.unitMode = "none";
    attribute.fixedUnit = "";
    attribute.unitOptions = [];
    attribute.unit = "";
  }

  attribute.value = value;
}

export function draftFromItem(
  item: ItemEditorResponse,
  template: ItemAttributeTemplateResponse | null = null,
): ItemDraft {
  const draft: ItemDraft = {
    id: item.id,
    name: item.name,
    sku: item.sku,
    categoryId: item.category_id,
    attributeTemplateId: item.attribute_template_id,
    unit: item.unit,
    image: {
      kind: "file",
      fileId: item.image_file_id,
      name: `${item.name} 主图`,
      mimeType: "image/*",
      sizeBytes: 0,
      status: "uploaded",
      progress: 100,
      error: "",
    },
    imageTemporary: false,
    obsoleteImageFileId: null,
    description: item.description ?? "",
    defaultPrice: item.default_price,
    reorderPoint: item.reorder_point,
    attributes: item.attributes.map((attribute) => ({
      key: crypto.randomUUID(),
      definitionId: attribute.definition_id,
      custom: attribute.custom,
      fieldName: attribute.field_name,
      fieldType: attribute.field_type,
      options: attribute.options ?? [],
      unitMode: attribute.unit_mode,
      fixedUnit: attribute.fixed_unit ?? "",
      unitOptions: attribute.unit_options ?? [],
      value:
        attribute.field_type === "file"
          ? {
              kind: "file",
              fileId: (attribute.value as FileAttributeReference).file_id,
              name: `图片 #${(attribute.value as FileAttributeReference).file_id}`,
              mimeType: "image/*",
              sizeBytes: 0,
              status: "uploaded",
              progress: 100,
              error: "",
            }
          : (attribute.value as string | number | boolean),
      unit: attribute.unit ?? "",
      fileTemporary: false,
    })),
  };
  if (template?.id === item.attribute_template_id) applyAttributeTemplate(draft, template);
  return draft;
}

export function applyAttributeTemplate(
  draft: ItemDraft,
  template: ItemAttributeTemplateResponse | null,
): void {
  if (template) {
    const customNames = new Set(
      draft.attributes
        .filter((attribute) => attribute.custom)
        .map((attribute) => attribute.fieldName.trim().toLowerCase()),
    );
    const conflicts = template.fields
      .filter((field) => customNames.has(field.field_name.toLowerCase()))
      .map((field) => field.field_name);
    if (conflicts.length > 0)
      throw new Error(`自定义属性与目标模板字段重名：${conflicts.join("、")}`);
  }
  const sameTemplate = template !== null && draft.attributeTemplateId === template.id;
  const existingTemplateAttributes = sameTemplate
    ? new Map(
        draft.attributes
          .filter((attribute) => !attribute.custom)
          .map((attribute) => [attribute.definitionId, attribute]),
      )
    : new Map<number | null, ItemAttributeDraft>();
  draft.attributeTemplateId = template?.id ?? null;
  draft.attributes = draft.attributes.filter((attribute) => attribute.custom);
  if (!template) return;
  for (const field of template.fields) {
    const current = existingTemplateAttributes.get(field.id);
    if (current) {
      current.definitionId = field.id;
      current.fieldName = field.field_name;
      if (current.fieldType !== field.field_type) {
        current.fieldType = field.field_type;
        current.value = initialFieldValue(field);
        current.unit = "";
      }
      applyTemplateUnit(current, field);
      draft.attributes.push(current);
      continue;
    }
    draft.attributes.push(attributeFromField(field));
  }
}

export function newCustomAttribute(): ItemAttributeDraft {
  return {
    key: crypto.randomUUID(),
    definitionId: null,
    custom: true,
    fieldName: "",
    fieldType: "text",
    options: [],
    unitMode: "none",
    fixedUnit: "",
    unitOptions: [],
    value: "",
    unit: "",
    fileTemporary: true,
  };
}

export function itemAttributeRequests(draft: ItemDraft): ItemAttributeRequest[] {
  return draft.attributes
    .filter(
      (attribute) =>
        attribute.fieldName.trim() && attribute.value !== undefined && attribute.value !== "",
    )
    .map((attribute) => ({
      definition_id: attribute.definitionId ?? undefined,
      field_name: attribute.fieldName.trim(),
      field_type: attribute.fieldType,
      options:
        attribute.custom && attribute.fieldType === "select"
          ? attribute.options.map((value) => value.trim())
          : undefined,
      unit_mode:
        attribute.custom && attribute.fieldType === "number" ? attribute.unitMode : undefined,
      fixed_unit:
        attribute.custom && attribute.unitMode === "fixed" ? attribute.fixedUnit.trim() : undefined,
      unit_options:
        attribute.custom && attribute.unitMode === "select"
          ? attribute.unitOptions.map((value) => value.trim())
          : undefined,
      value:
        attribute.fieldType === "file"
          ? { file_id: (attribute.value as FileDraftValue).fileId as number }
          : attribute.fieldType === "number"
            ? Number(attribute.value)
            : (attribute.value as string | boolean),
      unit: attribute.unit.trim() || undefined,
    }));
}

export interface ItemDraftValidationResult {
  errors: Record<string, string>;
  firstMessage: string;
}

/** 提交前生成字段级错误，替代浏览器原生约束气泡。 */
export function validateItemDraft(
  draft: ItemDraft,
  templates: ItemAttributeTemplateResponse[],
): ItemDraftValidationResult | null {
  const errors: Record<string, string> = {};
  if (!draft.name.trim()) errors.name = "请填写物品名称。";
  if (!draft.sku.trim()) errors.sku = "请填写编号。";
  if (!draft.unit.trim()) errors.unit = "请填写计量单位。";
  if (!draft.image) errors.image = "请选择物品主图。";
  if (
    draft.defaultPrice !== null &&
    (!Number.isFinite(draft.defaultPrice) || draft.defaultPrice < 0)
  ) {
    errors.defaultPrice = "参考单价必须是大于或等于 0 的有效数字。";
  }
  if (
    draft.reorderPoint !== null &&
    (!Number.isFinite(draft.reorderPoint) || draft.reorderPoint < 0)
  ) {
    errors.reorderPoint = "再订货点必须是大于或等于 0 的有效数字。";
  }

  const template = templates.find((candidate) => candidate.id === draft.attributeTemplateId);
  const templateFields = new Map(template?.fields.map((field) => [field.id, field]) ?? []);
  const names = new Set<string>();
  for (const attribute of draft.attributes) {
    const name = attribute.fieldName.trim();
    const normalizedName = name.toLowerCase();
    const prefix = `attribute.${attribute.key}`;
    if (attribute.custom && !name) errors[`${prefix}.name`] = "请填写属性名称。";
    if (name && !names.add(normalizedName)) errors[`${prefix}.name`] = `属性名称“${name}”重复。`;

    const field =
      attribute.definitionId === null ? undefined : templateFields.get(attribute.definitionId);
    if ((attribute.custom || field?.required) && !hasAttributeValue(attribute.value)) {
      errors[`${prefix}.value`] = `请填写“${name || field?.field_name || "未命名属性"}”的值。`;
    }
    if (
      attribute.fieldType === "number" &&
      attribute.value !== "" &&
      attribute.value !== undefined
    ) {
      const value = Number(attribute.value);
      if (!Number.isFinite(value)) errors[`${prefix}.value`] = "请输入有效数字。";
    }
    if (
      attribute.fieldType === "url" &&
      typeof attribute.value === "string" &&
      attribute.value.trim()
    ) {
      try {
        const url = new URL(attribute.value);
        if (!["http:", "https:"].includes(url.protocol))
          errors[`${prefix}.value`] = "请输入 HTTP 或 HTTPS 地址。";
      } catch {
        errors[`${prefix}.value`] = "请输入有效网址。";
      }
    }
    if (attribute.custom && attribute.fieldType === "select") {
      const optionsError = validateOptionList(attribute.options);
      if (optionsError) errors[`${prefix}.options`] = optionsError;
      if (
        typeof attribute.value === "string" &&
        !attribute.options.map((option) => option.trim()).includes(attribute.value)
      ) {
        errors[`${prefix}.value`] = "属性值必须来自候选项。";
      }
    }
    if (attribute.custom && attribute.fieldType === "number") {
      if (attribute.unitMode === "fixed" && !attribute.fixedUnit.trim()) {
        errors[`${prefix}.unitSettings`] = "请设置指定单位。";
      }
      if (attribute.unitMode === "select") {
        const optionsError = validateOptionList(attribute.unitOptions);
        if (optionsError) errors[`${prefix}.unitSettings`] = optionsError;
        if (!attribute.unit || !attribute.unitOptions.includes(attribute.unit)) {
          errors[`${prefix}.unitValue`] = "请选择实际单位。";
        }
      }
    }
  }
  const firstMessage = Object.values(errors)[0];
  return firstMessage ? { errors, firstMessage } : null;
}

/** 将物品接口返回的结构化错误映射回当前草稿字段，Notice 只保留为提交总提示。 */
export function itemDraftValidationFromApiError(
  error: ApiError,
  draft: ItemDraft,
): ItemDraftValidationResult | null {
  if (error.code === "sku_taken") {
    return { errors: { sku: "编号已存在，请更换。" }, firstMessage: "编号已存在，请更换。" };
  }

  const errors: Record<string, string> = {};
  const baseFields: Record<string, { key: string; message: string }> = {
    name: { key: "name", message: "请检查物品名称。" },
    sku: { key: "sku", message: "请检查编号。" },
    unit: { key: "unit", message: "请检查计量单位。" },
    image_file_id: { key: "image", message: "请重新选择物品主图。" },
    default_price: { key: "defaultPrice", message: "请检查参考单价。" },
    reorder_point: { key: "reorderPoint", message: "请检查再订货点。" },
  };
  for (const path of Object.keys(error.fieldErrors)) {
    const baseField = baseFields[path];
    if (baseField) {
      errors[baseField.key] = baseField.message;
      continue;
    }

    const attributePath = /^attributes(?:\[(\d+)\]|\.(\d+))\.(field_name|value|unit)$/.exec(path);
    if (!attributePath) continue;
    const attribute = draft.attributes[Number(attributePath[1] ?? attributePath[2])];
    if (!attribute) continue;
    const field =
      attributePath[3] === "field_name"
        ? "name"
        : attributePath[3] === "unit"
          ? "unitValue"
          : "value";
    errors[`attribute.${attribute.key}.${field}`] =
      attributePath[3] === "field_name"
        ? "请检查属性名称。"
        : attributePath[3] === "unit"
          ? "请检查属性单位。"
          : "请检查属性值。";
  }

  const firstMessage = Object.values(errors)[0];
  return firstMessage ? { errors, firstMessage } : null;
}

function hasAttributeValue(value: ItemAttributeDraft["value"]): boolean {
  if (typeof value === "string") return value.trim().length > 0;
  return value !== undefined && value !== null;
}

function validateOptionList(options: string[]): string | null {
  if (options.length === 0) return "至少添加一个候选项。";
  const names = new Set<string>();
  for (const option of options) {
    const normalized = option.trim().toLowerCase();
    if (!normalized) return "候选项不能为空。";
    if (!names.add(normalized)) return "候选项忽略大小写后不能重复。";
  }
  return null;
}

/** 把共享物品草稿转换为创建请求，供物品页和其它业务入口复用。 */
export function itemCreateRequest(draft: ItemDraft): ItemCreateRequest {
  return {
    name: draft.name.trim(),
    sku: draft.sku.trim(),
    unit: draft.unit.trim(),
    image_file_id: draft.image?.fileId as number,
    category_id: draft.categoryId ?? undefined,
    attribute_template_id: draft.attributeTemplateId ?? undefined,
    description: draft.description.trim() || undefined,
    default_price: draft.defaultPrice ?? undefined,
    reorder_point: draft.reorderPoint ?? undefined,
    attributes: itemAttributeRequests(draft),
  };
}

/** 按 OpenAPI 的部分更新语义只发送变化字段，并显式保留可清空字段的 null。 */
export function itemUpdateRequest(draft: ItemDraft, baseline: ItemDraft): ItemUpdateRequest {
  const request: ItemUpdateRequest = {};
  if (draft.name.trim() !== baseline.name.trim()) request.name = draft.name.trim();
  if (draft.sku.trim() !== baseline.sku.trim()) request.sku = draft.sku.trim();
  if (draft.unit.trim() !== baseline.unit.trim()) request.unit = draft.unit.trim();
  if (draft.image?.fileId !== baseline.image?.fileId) request.image_file_id = draft.image?.fileId;
  if (draft.categoryId !== baseline.categoryId) request.category_id = draft.categoryId;
  if (draft.attributeTemplateId !== baseline.attributeTemplateId)
    request.attribute_template_id = draft.attributeTemplateId;

  const description = draft.description.trim() || null;
  const baselineDescription = baseline.description.trim() || null;
  if (description !== baselineDescription) request.description = description;
  if (draft.defaultPrice !== baseline.defaultPrice) request.default_price = draft.defaultPrice;
  if (draft.reorderPoint !== baseline.reorderPoint) request.reorder_point = draft.reorderPoint;

  const attributes = itemAttributeRequests(draft);
  if (JSON.stringify(attributes) !== JSON.stringify(itemAttributeRequests(baseline))) {
    request.attributes = attributes;
  }
  return request;
}

/** 生成可比较的草稿快照，用于跨页面统一判断是否存在未保存修改。 */
export function itemDraftFingerprint(draft: ItemDraft): string {
  return JSON.stringify({
    id: draft.id,
    name: draft.name,
    sku: draft.sku,
    categoryId: draft.categoryId,
    attributeTemplateId: draft.attributeTemplateId,
    image: draft.image ? [draft.image.fileId, draft.image.name, draft.image.sizeBytes] : null,
    unit: draft.unit,
    description: draft.description,
    defaultPrice: draft.defaultPrice,
    reorderPoint: draft.reorderPoint,
    attributes: draft.attributes.map((attribute) => ({
      definitionId: attribute.definitionId,
      custom: attribute.custom,
      fieldName: attribute.fieldName,
      fieldType: attribute.fieldType,
      options: attribute.options,
      unitMode: attribute.unitMode,
      fixedUnit: attribute.fixedUnit,
      unitOptions: attribute.unitOptions,
      value:
        typeof attribute.value === "object" && attribute.value?.kind === "file"
          ? [attribute.value.fileId, attribute.value.name, attribute.value.sizeBytes]
          : attribute.value,
      unit: attribute.unit,
    })),
  });
}

function attributeFromField(field: ItemAttributeTemplateFieldResponse): ItemAttributeDraft {
  const attribute: ItemAttributeDraft = {
    key: crypto.randomUUID(),
    definitionId: field.id,
    custom: false,
    fieldName: field.field_name,
    fieldType: field.field_type,
    options: field.options ?? [],
    unitMode: field.unit.mode,
    fixedUnit: field.unit.value ?? "",
    unitOptions: field.unit.options ?? [],
    value: initialFieldValue(field),
    unit: "",
    fileTemporary: true,
  };
  applyTemplateUnit(attribute, field);
  return attribute;
}

function initialFieldValue(field: TemplateFieldResponse): ItemAttributeDraft["value"] {
  return field.default_value === null
    ? field.field_type === "boolean"
      ? undefined
      : ""
    : field.field_type === "number"
      ? Number(field.default_value)
      : field.field_type === "boolean"
        ? field.default_value === "true"
        : field.default_value;
}

function applyTemplateUnit(
  attribute: ItemAttributeDraft,
  field: ItemAttributeTemplateFieldResponse,
): void {
  const rule = field.unit;
  if (rule.mode === "fixed") attribute.unit = rule.value ?? "";
  else if (rule.mode === "none") attribute.unit = "";
  else if (rule.mode === "select" && !rule.options?.includes(attribute.unit)) attribute.unit = "";
}
