<!-- 本组件拥有手动输入立创商品编号的查询 Dialog；查询执行与候选确认内容复用共享模块，它不直接修改物品草稿。 -->
<template>
  <ModalDialog
    :open="open"
    title="查询立创资料"
    description="输入 C 开头的立创商城商品编号。"
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
        label="立创商品编号"
        name="lcsc_product_code"
        maxlength="32"
        autocomplete="off"
        autofocus
        placeholder="例如 C2983288"
        :disabled="request.pending.value"
        :error="inputError"
        :title="inputError || undefined"
        @update:model-value="inputError = ''"
      />

      <div v-if="request.pending.value" class="lcsc-lookup__status" role="status">
        正在查询立创资料…
      </div>
      <div v-else-if="request.error.value" class="lcsc-lookup__error" role="alert">
        <strong>查询失败</strong>
        <span>{{ request.error.value }}</span>
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
        <button class="secondary-button" type="button" @click="requestClose">不填写</button>
        <button class="primary-button" type="button" @click="applyCandidate">覆盖填写</button>
      </template>
      <template v-else>
        <button
          class="secondary-button"
          type="button"
          :disabled="request.pending.value"
          @click="requestClose"
        >
          取消
        </button>
        <button
          class="primary-button"
          type="submit"
          :form="formId"
          :disabled="request.pending.value"
        >
          {{ request.pending.value ? "正在查询…" : request.error.value ? "重新查询" : "查询" }}
        </button>
      </template>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { nextTick, ref, useId, watch } from "vue";
import type { ItemAttributeTemplateResponse } from "../../api/itemAttributeTemplates";
import type { LcscItemLookupResponse } from "../../api/items";
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

const formId = `lcsc-lookup-${useId()}`;
const productCode = ref("");
const inputError = ref("");
const selectedTemplateId = ref<number | null>(null);
const request = useLcscLookupRequest();

watch(
  () => props.open,
  async (open) => {
    request.reset();
    if (!open) return;
    productCode.value = normalizeCode(props.initialCode);
    inputError.value = "";
    selectedTemplateId.value = props.templates[0]?.id ?? null;
    await nextTick();
    focusCodeInput();
  },
);

watch(request.candidate, (candidate) => {
  if (candidate) selectedTemplateId.value = props.templates[0]?.id ?? null;
});

async function submitLookup(): Promise<void> {
  const normalized = normalizeCode(productCode.value);
  productCode.value = normalized;
  if (!/^C[0-9]+$/.test(normalized)) {
    inputError.value = "商品编号必须以 C 开头，后续全部为数字。";
    focusCodeInput();
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
