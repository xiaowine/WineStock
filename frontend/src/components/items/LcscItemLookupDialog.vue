<!-- 本组件拥有手动输入立创商品编号的查询 Dialog；查询执行与候选确认内容复用共享模块，它不直接修改物品草稿。 -->
<template>
  <ModalDialog
    :open="open"
    :title="$t('items.lcscLookupTitle')"
    :description="$t('items.lcscLookupDescription')"
    compact
    nested
    @close="requestClose"
  >
    <form
      v-if="!request.candidate.value"
      :id="formId"
      class="lcsc-lookup"
      novalidate
      @submit.prevent="submitLookup"
    >
      <FormInput
        :id="`${formId}-product-code`"
        v-model="productCode"
        :label="$t('items.lcscProductCode')"
        name="lcsc_product_code"
        maxlength="32"
        autocomplete="off"
        autofocus
        :placeholder="$t('items.lcscCodePlaceholder')"
        :disabled="request.pending.value"
        validation-key="productCode"
        :error="errors.productCode"
      />

      <div v-if="request.pending.value" class="lcsc-lookup__status" role="status">
        {{ $t('items.lcscQuerying') }}
      </div>
    </form>

    <LcscCandidateConfirmPanel
      v-else
      v-model:template-id="selectedTemplateId"
      :candidate="request.candidate.value"
      :templates="templates"
    />

    <template #actions>
      <template v-if="request.candidate.value">
        <button class="secondary-button" type="button" @click="requestClose">
          {{ $t('items.doNotFill') }}
        </button>
        <button class="primary-button" type="button" @click="applyCandidate">
          {{ $t('items.overwriteFill') }}
        </button>
      </template>
      <template v-else>
        <button
          class="secondary-button"
          type="button"
          :disabled="request.pending.value"
          @click="requestClose"
        >
          {{ $t('common.cancel') }}
        </button>
        <button
          class="primary-button"
          type="submit"
          :form="formId"
          :disabled="request.pending.value"
        >
          {{
            request.pending.value
              ? $t('items.querying')
              : request.error.value
                ? $t('items.reQuery')
                : $t('items.query')
          }}
        </button>
      </template>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { nextTick, ref, useId, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { ItemAttributeTemplateResponse } from "../../api/itemAttributeTemplates";
import type { LcscItemLookupResponse } from "../../api/items";
import { defaultAttributeTemplate } from "../../pages/items/model";
import { useFormValidation } from "../../composables/useFormValidation";
import { notice } from "../../notices/notice";
import FormInput from "../forms/FormInput.vue";
import ModalDialog from "../ModalDialog.vue";
import LcscCandidateConfirmPanel from "./LcscCandidateConfirmPanel.vue";
import { useLcscLookupRequest } from "./useLcscLookupRequest";

const props = withDefaults(
  defineProps<{
    open: boolean;
    initialCode?: string;
    templates: ItemAttributeTemplateResponse[];
  }>(),
  { initialCode: "" },
);

const emit = defineEmits<{
  close: [];
  apply: [candidate: LcscItemLookupResponse, templateId: number | null];
}>();
const { t } = useI18n();

const formId = `lcsc-lookup-${useId()}`;
const productCode = ref("");
const errors = ref<Record<string, string>>({});
const { clearErrors } = useFormValidation(errors);
const selectedTemplateId = ref<number | null>(null);
const request = useLcscLookupRequest();

watch(
  () => props.open,
  async (open) => {
    request.reset();
    if (!open) return;
    productCode.value = normalizeCode(props.initialCode);
    clearErrors();
    selectedTemplateId.value = preferredTemplateId();
    await nextTick();
    focusCodeInput();
  },
);

watch(request.candidate, (candidate) => {
  if (candidate) selectedTemplateId.value = preferredTemplateId();
});

watch(request.error, (error) => {
  if (error) {
    notice.error(t("items.lcscQueryFailed"), {
      detail: error,
      onClick: () => void submitLookup(),
    });
  }
});

/** 模板预选：全站默认优先，未设置时退回列表第一项。 */
function preferredTemplateId(): number | null {
  return (defaultAttributeTemplate(props.templates) ?? props.templates[0])?.id ?? null;
}

async function submitLookup(): Promise<void> {
  const normalized = normalizeCode(productCode.value);
  productCode.value = normalized;
  if (!/^C[0-9]+$/.test(normalized)) {
    const error = t("items.lcscCodeInvalid");
    errors.value = { productCode: error };
    notice.warning(t("items.checkLcscCode"), { detail: error });
    return;
  }
  await request.lookup(normalized);
}

function requestClose(): void {
  request.abort();
  emit("close");
}

function applyCandidate(): void {
  const candidate = request.candidate.value;
  if (!candidate) return;
  emit("apply", candidate, selectedTemplateId.value);
  emit("close");
}

function normalizeCode(value: string): string {
  return value.trim().toUpperCase();
}

function focusCodeInput(): void {
  document.getElementById(`${formId}-product-code`)?.focus();
}
</script>

<style lang="scss" src="./LcscItemLookupDialog.scss"></style>
