import type {
  ItemAttributeTemplateFieldRequest,
  ItemAttributeTemplateFieldResponse,
  ItemAttributeTemplateResponse,
  ItemAttributeUnitMode,
  ItemAttributeUnitRule,
} from "../../api/itemAttributeTemplates";
import type {
  TemplateFieldRequest,
  TemplateFieldResponse,
  TemplateFieldType,
} from "../../api/templateFields";
import { i18n, translateMessageOrNull } from "../../i18n";
import type { MessageKeyPath } from "../../i18n/schema";

/** 按消息键取模板域校验文案；带参数时按 {name} 命名插值。 */
function localizedMessage(key: string, params?: Record<string, string | number>): string {
  if (!params) return translateMessageOrNull(key) ?? key;
  return i18n.global.t(key as MessageKeyPath, params);
}

export type TemplateDomain = "category" | "item";

export interface TemplateFieldDraft {
  key: string;
  definitionId: number | null;
  fieldName: string;
  fieldType: TemplateFieldType;
  defaultValue: string;
  options: string[];
  required: boolean;
  searchable: boolean;
  catalogVisible: boolean;
  unitMode: ItemAttributeUnitMode;
  unitValue: string;
  unitOptions: string[];
  expanded: boolean;
}

export interface TemplateDraft {
  name: string;
  description: string;
  fields: TemplateFieldDraft[];
}

export interface TemplateDraftValidation {
  errors: Record<string, string>;
  firstFieldIndex: number | null;
}

let fieldKeySequence = 0;

export function createEmptyField(catalogVisible = false): TemplateFieldDraft {
  fieldKeySequence += 1;
  return {
    key: `template-field-${fieldKeySequence}`,
    definitionId: null,
    fieldName: "",
    fieldType: "text",
    defaultValue: "",
    options: [],
    required: false,
    searchable: false,
    catalogVisible,
    unitMode: "none",
    unitValue: "",
    unitOptions: [],
    expanded: true,
  };
}

export function createTemplateDraft(template: ItemAttributeTemplateResponse | null): TemplateDraft {
  if (!template) {
    return {
      name: "",
      description: "",
      fields: [createEmptyField(true)],
    };
  }

  return {
    name: template.name,
    description: template.description ?? "",
    fields: [...template.fields]
      .sort((left, right) => left.sort_order - right.sort_order)
      .map(responseFieldToDraft),
  };
}

export function serializeTemplateDraft(draft: TemplateDraft): string {
  return JSON.stringify({
    ...draft,
    fields: draft.fields.map(({ expanded: _expanded, ...field }) => field),
  });
}

export function validateTemplateDraft(draft: TemplateDraft): TemplateDraftValidation {
  const errors: Record<string, string> = {};
  const name = draft.name.trim();
  const description = draft.description.trim();
  if (!name) errors.name = localizedMessage("templates.modelNameRequired");
  else if (name.length > 128) errors.name = localizedMessage("templates.nameTooLong");
  if (description.length > 1024) errors.description = localizedMessage("templates.descriptionTooLong");
  if (draft.fields.length < 1) errors.fields = localizedMessage("templates.atLeastOneField");
  if (draft.fields.length > 64) errors.fields = localizedMessage("templates.maxFields");

  const names = new Map<string, number>();
  let catalogVisibleCount = 0;
  let firstFieldIndex: number | null = null;
  draft.fields.forEach((field, index) => {
    const prefix = `fields.${index}`;
    const fieldName = field.fieldName.trim();
    if (!fieldName) errors[`${prefix}.field_name`] = localizedMessage("templates.fieldNameRequired");
    else if (fieldName.length > 64)
      errors[`${prefix}.field_name`] = localizedMessage("templates.fieldNameTooLong");
    else {
      const normalized = fieldName.toLocaleLowerCase();
      if (names.has(normalized)) {
        errors[`${prefix}.field_name`] = localizedMessage("templates.fieldNameDuplicate");
        const previous = names.get(normalized);
        if (previous !== undefined)
          errors[`fields.${previous}.field_name`] = localizedMessage("templates.fieldNameDuplicate");
      } else names.set(normalized, index);
    }

    validateDefaultValue(field, prefix, errors);
    if (field.fieldType === "select")
      validateStringOptions(field.options, prefix, "options", 128, 128, errors);
    if (field.catalogVisible) catalogVisibleCount += 1;
    validateUnit(field, prefix, errors);
    if (
      firstFieldIndex === null &&
      Object.keys(errors).some((key) => key.startsWith(`${prefix}.`))
    ) {
      firstFieldIndex = index;
    }
  });

  if (catalogVisibleCount > 3) errors.catalog_visible = localizedMessage("templates.maxCatalogVisible");
  return { errors, firstFieldIndex };
}

export function buildItemTemplateRequest(draft: TemplateDraft) {
  return {
    name: draft.name.trim(),
    description: nullableTrimmed(draft.description),
    fields: draft.fields.map<ItemAttributeTemplateFieldRequest>((field) => ({
      definition_id: field.definitionId,
      ...buildBaseFieldRequest(field),
      catalog_visible: field.catalogVisible,
      unit: buildUnitRule(field),
    })),
  };
}

export function clearIncompatibleFieldData(
  field: TemplateFieldDraft,
  nextType: TemplateFieldType,
): void {
  field.fieldType = nextType;
  if (nextType !== "select") field.options = [];
  if (nextType === "file") field.defaultValue = "";
  if (nextType === "boolean" && !["true", "false"].includes(field.defaultValue))
    field.defaultValue = "";
  if (nextType === "select" && !field.options.includes(field.defaultValue)) field.defaultValue = "";
}

export function fieldTypeLabel(type: TemplateFieldType): string {
  const key = {
    text: "templates.typeText",
    number: "templates.typeNumber",
    select: "templates.typeSelect",
    date: "templates.typeDate",
    file: "templates.typeFile",
    url: "templates.typeUrl",
    boolean: "templates.typeBoolean",
  }[type];
  return translateMessageOrNull(key) ?? key;
}

function responseFieldToDraft(field: TemplateFieldResponse): TemplateFieldDraft {
  const itemField = field as ItemAttributeTemplateFieldResponse;
  fieldKeySequence += 1;
  return {
    key: `template-field-${field.id}-${fieldKeySequence}`,
    definitionId: field.id,
    fieldName: field.field_name,
    fieldType: field.field_type,
    defaultValue: field.default_value ?? "",
    options: [...(field.options ?? [])],
    required: field.required,
    searchable: field.searchable,
    catalogVisible: itemField.catalog_visible ?? false,
    unitMode: itemField.unit?.mode ?? "none",
    unitValue: itemField.unit?.value ?? "",
    unitOptions: [...(itemField.unit?.options ?? [])],
    expanded: true,
  };
}

function buildBaseFieldRequest(field: TemplateFieldDraft): TemplateFieldRequest {
  return {
    field_name: field.fieldName.trim(),
    field_type: field.fieldType,
    default_value: field.fieldType === "file" ? null : nullableTrimmed(field.defaultValue),
    options: field.fieldType === "select" ? field.options.map((option) => option.trim()) : null,
    required: field.required,
    searchable: field.searchable,
  };
}

function buildUnitRule(field: TemplateFieldDraft): ItemAttributeUnitRule {
  if (field.unitMode === "fixed")
    return { mode: "fixed", value: field.unitValue.trim(), options: null };
  if (field.unitMode === "select")
    return {
      mode: "select",
      value: null,
      options: field.unitOptions.map((option) => option.trim()),
    };
  return { mode: "none", value: null, options: null };
}

function validateDefaultValue(
  field: TemplateFieldDraft,
  prefix: string,
  errors: Record<string, string>,
): void {
  const value = field.defaultValue.trim();
  if (value.length > 256) errors[`${prefix}.default_value`] = localizedMessage("templates.defaultValueTooLong");
  if (!value) return;
  if (field.fieldType === "number" && !Number.isFinite(Number(value)))
    errors[`${prefix}.default_value`] = localizedMessage("templates.invalidNumber");
  if (field.fieldType === "boolean" && !["true", "false"].includes(value))
    errors[`${prefix}.default_value`] = localizedMessage("templates.booleanDefaultInvalid");
  if (field.fieldType === "url" && !isHttpUrl(value))
    errors[`${prefix}.default_value`] = localizedMessage("templates.urlInvalid");
  if (field.fieldType === "date" && !isCalendarDate(value))
    errors[`${prefix}.default_value`] = localizedMessage("templates.dateInvalid");
  if (field.fieldType === "file")
    errors[`${prefix}.default_value`] = localizedMessage("templates.fileNoDefault");
  if (field.fieldType === "select" && !field.options.some((option) => option.trim() === value)) {
    errors[`${prefix}.default_value`] = localizedMessage("templates.defaultNotInOptions");
  }
}

function validateUnit(
  field: TemplateFieldDraft,
  prefix: string,
  errors: Record<string, string>,
): void {
  if (field.unitMode === "fixed") {
    const value = field.unitValue.trim();
    if (!value) errors[`${prefix}.unit_value`] = localizedMessage("templates.unitValueRequired");
    else if (value.length > 32)
      errors[`${prefix}.unit_value`] = localizedMessage("templates.unitValueTooLong");
  }
  if (field.unitMode === "select") {
    validateStringOptions(field.unitOptions, prefix, "unit_options", 32, 32, errors);
  }
}

function validateStringOptions(
  options: readonly string[],
  prefix: string,
  key: string,
  maxItems: number,
  maxLength: number,
  errors: Record<string, string>,
): void {
  if (options.length < 1) errors[`${prefix}.${key}`] = localizedMessage("templates.atLeastOneOption");
  if (options.length > maxItems)
    errors[`${prefix}.${key}`] = localizedMessage("templates.maxOptions", { n: maxItems });
  const seen = new Set<string>();
  options.forEach((option, optionIndex) => {
    const value = option.trim();
    const errorKey = `${prefix}.${key}.${optionIndex}`;
    if (!value) errors[errorKey] = localizedMessage("templates.optionRequired");
    else if (value.length > maxLength)
      errors[errorKey] = localizedMessage("templates.optionTooLong", { n: maxLength });
    const normalized = value.toLocaleLowerCase();
    if (value && seen.has(normalized)) errors[errorKey] = localizedMessage("templates.optionDuplicate");
    seen.add(normalized);
  });
}

function nullableTrimmed(value: string): string | null {
  const trimmed = value.trim();
  return trimmed || null;
}

function isHttpUrl(value: string): boolean {
  try {
    return ["http:", "https:"].includes(new URL(value).protocol);
  } catch {
    return false;
  }
}

function isCalendarDate(value: string): boolean {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) return false;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const date = new Date(Date.UTC(year, month - 1, day));
  return (
    date.getUTCFullYear() === year && date.getUTCMonth() === month - 1 && date.getUTCDate() === day
  );
}
