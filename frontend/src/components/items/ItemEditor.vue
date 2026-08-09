<!-- 本组件拥有物品基础资料、主图（含 Dialog 级快捷粘贴）和任意属性的表单布局；除临时图片清理外不发起 HTTP 请求。 -->
<template>
  <form
    :id="formId"
    ref="formRoot"
    class="item-editor"
    :class="{ 'item-editor--embedded': embedded, 'item-editor--readonly': readOnly }"
    novalidate
    @submit.prevent="submit"
  >
    <header v-if="!embedded" class="item-editor__header">
      <button
        class="icon-button item-editor__back"
        type="button"
        :title="closeLabelText"
        :aria-label="closeLabelText"
        @click="emit('close')"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="m15 5-7 7 7 7" />
        </svg>
      </button>
      <div class="item-editor__heading">
        <h2>{{ draft.id ? draft.name || $t('items.editItem') : $t('items.createItem') }}</h2>
        <p v-if="draft.id">{{ draft.sku }} · {{ draft.unit }}</p>
      </div>
      <button
        v-if="!readOnly"
        class="primary-button item-editor__desktop-save"
        type="submit"
        :disabled="saving"
      >
        {{ saving ? $t('items.saving') : $t('items.saveItem') }}
      </button>
    </header>

    <fieldset v-overlay-scrollbar class="item-editor__content" :disabled="readOnly">
      <div v-if="metadataError" class="item-editor__metadata-error" role="alert">
        {{ $t('items.metadataUnavailable') }}
      </div>

      <section class="item-editor__section" aria-labelledby="item-base-heading">
        <header class="item-editor__section-header">
          <h3 id="item-base-heading">{{ $t('items.basicInfo') }}</h3>
        </header>

        <div class="item-editor__base-layout">
          <FormField
            class="item-editor__image"
            :label="$t('items.itemMainImage')"
            validation-key="image"
            :error="validationErrors.image"
            required
          >
            <AttributeImageField
              :model-value="draft.image ?? undefined"
              :delete-on-remove="draft.imageTemporary"
              :invalid="Boolean(validationErrors.image)"
              :label="$t('items.itemMainImage')"
              @update:model-value="updateMainImage"
            />
            <p v-if="!readOnly" class="item-editor__paste-hint">{{ $t('items.pasteHint') }}</p>
          </FormField>

          <div class="item-editor__fields">
            <FormInput
              v-model="draft.name"
              :label="$t('items.name')"
              name="name"
              maxlength="128"
              autocomplete="off"
              validation-key="name"
              :error="validationErrors.name"
              required
            />
            <FormInput
              v-model="draft.sku"
              :label="$t('items.sku')"
              name="sku"
              maxlength="64"
              autocomplete="off"
              validation-key="sku"
              :error="validationErrors.sku"
              required
            />
            <FormSelect v-model="draft.categoryId" :label="$t('items.category')" name="category">
              <option :value="null">{{ $t('items.uncategorized') }}</option>
              <option v-for="category in categories" :key="category.id" :value="category.id">
                {{ category.name }}
              </option>
            </FormSelect>
            <FormInput
              v-model="draft.unit"
              :label="$t('items.unit')"
              name="unit"
              maxlength="32"
              autocomplete="off"
              validation-key="unit"
              :error="validationErrors.unit"
              required
            />
            <FormInput
              v-model="draft.defaultPrice"
              :label="$t('items.referencePrice')"
              name="default_price"
              type="number"
              min="0"
              step="0.01"
              inputmode="decimal"
              validation-key="defaultPrice"
              :error="validationErrors.defaultPrice"
            />
            <FormInput
              v-model="draft.reorderPoint"
              :label="$t('items.reorderPoint')"
              name="reorder_point"
              type="number"
              min="0"
              step="0.01"
              inputmode="decimal"
              validation-key="reorderPoint"
              :error="validationErrors.reorderPoint"
            />
            <FormTextarea
              v-model="draft.description"
              class="item-editor__description"
              :label="$t('items.description')"
              name="description"
              maxlength="1024"
              rows="3"
            />
          </div>
        </div>
      </section>

      <section
        class="item-editor__section item-editor__attributes"
        aria-labelledby="item-attributes-heading"
      >
        <header class="item-editor__section-header">
          <h3 id="item-attributes-heading">{{ $t('items.itemAttributes') }}</h3>
        </header>

        <FormSelect
          class="item-editor__template"
          :label="$t('items.attributeTemplate')"
          name="attribute_template"
          :model-value="draft.attributeTemplateId ?? ''"
          @update:model-value="selectTemplate"
        >
          <option value="">{{ $t('items.noTemplate') }}</option>
          <option v-for="template in templates" :key="template.id" :value="template.id">
            {{ template.name }}
          </option>
        </FormSelect>

        <section
          v-if="templateAttributes.length"
          class="item-editor__attribute-group"
          aria-labelledby="item-template-attributes-heading"
        >
          <header class="item-editor__attribute-group-header">
            <h4 id="item-template-attributes-heading">{{ $t('items.templateAttributes') }}</h4>
          </header>
          <div class="item-editor__template-attributes">
            <ItemAttributeEditor
              v-for="attribute in templateAttributes"
              :key="attribute.key"
              :attribute="attribute"
              :template-field="templateFieldsById.get(attribute.definitionId ?? -1)"
              :validation-errors="attributeValidationErrors(attribute.key)"
            />
          </div>
        </section>

        <section
          class="item-editor__attribute-group"
          aria-labelledby="item-custom-attributes-heading"
        >
          <header class="item-editor__attribute-group-header">
            <h4 id="item-custom-attributes-heading">{{ $t('items.customAttributes') }}</h4>
            <button
              class="secondary-button item-editor__add-attribute"
              type="button"
              @click="addAttribute"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 5v14M5 12h14" />
              </svg>
              {{ $t('items.addAttribute') }}
            </button>
          </header>
          <div v-if="customAttributes.length" class="item-editor__custom-attributes">
            <ItemAttributeEditor
              v-for="attribute in customAttributes"
              :key="attribute.key"
              :attribute="attribute"
              :validation-errors="attributeValidationErrors(attribute.key)"
              @remove="removeAttribute(attribute.key)"
            />
          </div>
        </section>
      </section>
    </fieldset>

    <footer v-if="!embedded" class="item-editor__mobile-actions">
      <button v-if="!readOnly" class="primary-button" type="submit" :disabled="saving">
        {{ saving ? $t('items.saving') : $t('items.saveItem') }}
      </button>
    </footer>
  </form>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { ItemCategoryResponse } from "../../api/itemCategories";
import type { ItemAttributeTemplateResponse } from "../../api/itemAttributeTemplates";
import { deleteImage, validateImageFile } from "../../api/files";
import {
  applyAttributeTemplate,
  newCustomAttribute,
  type ItemDraft,
} from "../../pages/items/model";
import { discardTemporaryAttributeFile } from "../../pages/items/fileCleanup";
import ItemAttributeEditor from "./ItemAttributeEditor.vue";
import AttributeImageField from "../attributes/AttributeImageField.vue";
import {
  createPendingImageDraft,
  extractClipboardImageFile,
  type ImageDraftValue,
} from "../attributes/imageDraft";
import { notice } from "../../notices/notice";
import FormField from "../forms/FormField.vue";
import FormInput from "../forms/FormInput.vue";
import FormSelect from "../forms/FormSelect.vue";
import FormTextarea from "../forms/FormTextarea.vue";

const props = withDefaults(
  defineProps<{
    draft: ItemDraft;
    categories: ItemCategoryResponse[];
    templates: ItemAttributeTemplateResponse[];
    saving: boolean;
    metadataError: string;
    validationErrors: Record<string, string>;
    closeLabel?: string;
    embedded?: boolean;
    formId?: string;
    /** 没有物品管理权限时只展示资料，不允许修改草稿或提交。 */
    readOnly?: boolean;
  }>(),
  {
    closeLabel: undefined,
    embedded: false,
    formId: undefined,
    readOnly: false,
  },
);

const emit = defineEmits<{ save: []; close: [] }>();
const { t } = useI18n();
const closeLabelText = computed(() => props.closeLabel ?? t("items.backToCatalog"));
const formRoot = ref<HTMLFormElement | null>(null);
const templateFieldsById = computed(
  () =>
    new Map(
      props.templates.flatMap((template) =>
        template.fields.map((field) => [field.id, field] as const),
      ),
    ),
);
const templateAttributes = computed(() =>
  props.draft.attributes.filter((attribute) => !attribute.custom),
);
const customAttributes = computed(() =>
  props.draft.attributes.filter((attribute) => attribute.custom),
);

function submit(): void {
  if (!props.readOnly) emit("save");
}

function attributeValidationErrors(key: string): Record<string, string> {
  const prefix = `attribute.${key}.`;
  return Object.fromEntries(
    Object.entries(props.validationErrors)
      .filter(([field]) => field.startsWith(prefix))
      .map(([field, message]) => [field.slice(prefix.length), message]),
  );
}

function addAttribute(): void {
  props.draft.attributes.push(newCustomAttribute());
}

function removeAttribute(key: string): void {
  const index = props.draft.attributes.findIndex((attribute) => attribute.key === key);
  if (index >= 0) props.draft.attributes.splice(index, 1);
}

function updateMainImage(value: ImageDraftValue | undefined): void {
  if (!props.draft.imageTemporary && props.draft.image?.fileId) {
    props.draft.obsoleteImageFileId = props.draft.image.fileId;
  }
  props.draft.image = value ?? null;
  props.draft.imageTemporary = true;
}

onMounted(() => window.addEventListener("paste", handleGlobalPaste));
onBeforeUnmount(() => window.removeEventListener("paste", handleGlobalPaste));

/** Dialog 级快捷粘贴：把剪贴板中的图片直接设为物品主图。 */
function handleGlobalPaste(event: ClipboardEvent): void {
  if (props.readOnly || props.saving) return;
  if (!isTopmostPasteContext()) return;
  const file = extractClipboardImageFile(event);
  if (!file) return;
  // 焦点在文本控件且剪贴板同时携带文本时（如复制网页图文），保持默认文本粘贴。
  if (isTextEntryTarget(event.target) && event.clipboardData?.types.includes("text/plain")) return;
  event.preventDefault();
  void applyPastedImage(file);
}

/** 编辑器所在浮层必须是最上层，避免立创查询等嵌套 Dialog 中的粘贴误改主图。 */
function isTopmostPasteContext(): boolean {
  const layers = document.querySelectorAll(".modal-layer");
  const ownLayer = formRoot.value?.closest(".modal-layer") ?? null;
  if (!ownLayer) return layers.length === 0;
  return ownLayer === layers.item(layers.length - 1);
}

function isTextEntryTarget(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  );
}

async function applyPastedImage(file: File): Promise<void> {
  const error = await validateImageFile(file);
  if (error) {
    notice.warning(t("items.pasteImageFailed"), { detail: error });
    return;
  }
  // 与 AttributeImageField 的更换行为一致：临时上传文件立即删除，旧预览由字段监听释放。
  const current = props.draft.image;
  if (current?.fileId && props.draft.imageTemporary) {
    void deleteImage(current.fileId).catch(() => undefined);
  }
  updateMainImage(createPendingImageDraft(file));
  notice.success(t("items.pastedImageAsMain"));
}

async function selectTemplate(value: string | number | boolean | null | undefined): Promise<void> {
  const id = Number(value);
  const template = props.templates.find((candidate) => candidate.id === id) ?? null;
  const customNames = new Set(
    customAttributes.value.map((attribute) => attribute.fieldName.trim().toLowerCase()),
  );
  const conflicts =
    template?.fields
      .filter((field) => customNames.has(field.field_name.toLowerCase()))
      .map((field) => field.field_name) ?? [];
  if (conflicts.length > 0) {
    notice.warning(t("items.templateSwitchFailed"), {
      detail: t("items.templateConflictDetail", {
        names: conflicts.join(t("items.listSeparator")),
      }),
    });
    return;
  }
  const changingFiles = props.draft.attributes.filter(
    (attribute) => !attribute.custom && attribute.fieldType === "file",
  );
  try {
    await Promise.all(changingFiles.map(discardTemporaryAttributeFile));
  } catch {
    notice.warning(t("items.tempImagesCleanupFailed"), { detail: t("items.autoCleanupNote") });
  }
  applyAttributeTemplate(props.draft, template);
}
</script>

<style lang="scss" src="./ItemEditor.scss"></style>
