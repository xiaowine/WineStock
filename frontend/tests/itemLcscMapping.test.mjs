import assert from "node:assert/strict";
import test from "node:test";

import { applyLcscLookupToDraft, emptyItemDraft } from "../src/pages/items/model.ts";

function lookup(overrides = {}) {
  return {
    source: "lcsc",
    product_code: "C2983288",
    name: "BER-04",
    description: "旋转编码开关",
    manufacturer: "SM Switch",
    manufacturer_part: "BER-04",
    footprint: "插件",
    datasheet_url: "https://example.com/BER-04.pdf",
    default_price: 9.91,
    parameters: [{ name: "Operating Temperature", value: "-40℃~+85℃" }],
    ...overrides,
  };
}

function template() {
  const field = (id, field_name, field_type = "text") => ({
    id,
    field_name,
    field_type,
    default_value: null,
    options: null,
    required: false,
    searchable: true,
    sort_order: id,
    unit: { mode: "none", value: null, options: null },
    catalog_visible: false,
  });
  return {
    id: 21,
    name: "电子元器件",
    description: null,
    default_inbound_template_id: null,
    item_usage_count: 0,
    fields: [field(101, "型号"), field(102, "品牌"), field(103, "封装")],
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
  };
}

test("confirmed LCSC data overwrites matching fields and preserves local-only item settings", () => {
  const draft = emptyItemDraft();
  draft.name = "手动名称";
  draft.sku = "OLD-SKU";
  draft.description = "手动描述";
  draft.categoryId = 7;
  draft.attributeTemplateId = 11;
  draft.unit = "只";
  draft.defaultPrice = 12.5;
  draft.reorderPoint = 20;
  draft.image = {
    kind: "file",
    fileId: 99,
    name: "existing.png",
    mimeType: "image/png",
    sizeBytes: 1,
    status: "uploaded",
    progress: 100,
    error: "",
  };
  draft.attributes.push({
    key: "brand",
    definitionId: 15,
    custom: false,
    fieldName: "品牌",
    fieldType: "text",
    options: [],
    unitMode: "none",
    fixedUnit: "",
    unitOptions: [],
    value: "旧品牌",
    unit: "",
    fileTemporary: true,
  });
  draft.attributes.push({
    key: "model",
    definitionId: 16,
    custom: false,
    fieldName: "型号",
    fieldType: "select",
    options: ["旧型号"],
    unitMode: "none",
    fixedUnit: "",
    unitOptions: [],
    value: "旧型号",
    unit: "",
    fileTemporary: true,
  });

  applyLcscLookupToDraft(draft, lookup(), template());

  assert.equal(draft.name, "BER-04");
  assert.equal(draft.sku, "C2983288");
  assert.equal(draft.description, "旋转编码开关");
  assert.equal(draft.categoryId, 7);
  assert.equal(draft.attributeTemplateId, 21);
  assert.equal(draft.unit, "只");
  assert.equal(draft.defaultPrice, 9.91);
  assert.equal(draft.reorderPoint, 20);
  assert.equal(draft.image.fileId, 99);
  assert.equal(
    draft.attributes.find((attribute) => attribute.fieldName === "品牌").value,
    "SM Switch",
  );
  assert.equal(
    draft.attributes.find((attribute) => attribute.fieldName === "型号").value,
    "BER-04",
  );
  assert.equal(
    draft.attributes.find((attribute) => attribute.fieldName === "型号").definitionId,
    101,
  );
  assert.equal(draft.attributes.find((attribute) => attribute.fieldName === "型号").custom, false);
  assert.equal(
    draft.attributes.find((attribute) => attribute.fieldName === "参数").value,
    "Operating Temperature：-40℃~+85℃",
  );
});

test("missing LCSC values do not clear existing draft values", () => {
  const draft = emptyItemDraft();
  draft.name = "当前名称";
  draft.description = "当前描述";
  draft.attributes.push({
    key: "footprint",
    definitionId: null,
    custom: true,
    fieldName: "封装",
    fieldType: "text",
    options: [],
    unitMode: "none",
    fixedUnit: "",
    unitOptions: [],
    value: "SMD",
    unit: "",
    fileTemporary: true,
  });

  applyLcscLookupToDraft(
    draft,
    lookup({
      name: "",
      description: null,
      manufacturer: null,
      manufacturer_part: null,
      footprint: null,
      datasheet_url: null,
      default_price: null,
      parameters: [],
    }),
    null,
  );

  assert.equal(draft.sku, "C2983288");
  assert.equal(draft.name, "当前名称");
  assert.equal(draft.description, "当前描述");
  assert.equal(draft.defaultPrice, null);
  assert.equal(draft.attributes.find((attribute) => attribute.fieldName === "封装").value, "SMD");
});
