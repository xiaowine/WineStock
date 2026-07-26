<!--
  本组件拥有扫码/自动路径的立创查询流 Dialog：打开即以给定 C 号查询，只有 查询中、候选确认、错误 三态，
  没有手动输入表单。它不解析扫码内容、不写入草稿，取消与失败的去向由调用方决定。
-->
<template>
  <ModalDialog
    :open="open"
    title="立创资料"
    :description="`商品编号 ${productCode}`"
    compact
    nested
    @close="emit('dismiss')"
  >
    <LcscCandidateConfirmPanel
      v-if="request.candidate.value"
      v-model:template-id="selectedTemplateId"
      :candidate="request.candidate.value"
      :templates="templates"
    />
    <div v-else-if="request.error.value" class="lcsc-flow__error" role="alert">
      <strong>查询失败</strong>
      <span>{{ request.error.value }}</span>
    </div>
    <div v-else class="lcsc-flow__status" role="status">正在查询立创资料…</div>

    <template #actions>
      <template v-if="request.candidate.value">
        <button class="secondary-button" type="button" @click="emit('dismiss')">不填写</button>
        <button class="primary-button" type="button" @click="applyCandidate">覆盖填写</button>
      </template>
      <template v-else-if="request.error.value">
        <button class="secondary-button" type="button" @click="emit('dismiss')">
          {{ dismissLabel }}
        </button>
        <button class="primary-button" type="button" @click="retry">重试</button>
      </template>
      <button v-else class="secondary-button" type="button" @click="emit('dismiss')">
        {{ dismissLabel }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import type { ItemAttributeTemplateResponse } from "../../api/itemAttributeTemplates";
import type { LcscItemLookupResponse } from "../../api/items";
import ModalDialog from "../ModalDialog.vue";
import LcscCandidateConfirmPanel from "./LcscCandidateConfirmPanel.vue";
import { useLcscLookupRequest } from "./useLcscLookupRequest";

const props = withDefaults(
  defineProps<{
    open: boolean;
    /** 待查询的立创商品编号；打开时立即执行查询。 */
    productCode: string;
    templates: ItemAttributeTemplateResponse[];
    /** 取消按钮文案；扫码路径传「返回扫码」明确去向。 */
    dismissLabel?: string;
  }>(),
  { dismissLabel: "取消" },
);

const emit = defineEmits<{
  apply: [candidate: LcscItemLookupResponse, templateId: number | null];
  /** 用户取消、关闭或在错误态放弃；由调用方决定回到扫码还是留在编辑器。 */
  dismiss: [];
}>();

const request = useLcscLookupRequest();
const selectedTemplateId = ref<number | null>(null);

watch(
  () => props.open,
  (open) => {
    if (!open) {
      request.abort();
      return;
    }
    selectedTemplateId.value = props.templates[0]?.id ?? null;
    request.reset();
    void request.lookup(props.productCode);
  },
);

function retry(): void {
  void request.lookup(props.productCode);
}

function applyCandidate(): void {
  const candidate = request.candidate.value;
  if (!candidate) return;
  emit("apply", candidate, selectedTemplateId.value);
}
</script>

<style scoped lang="scss">
.lcsc-flow__status,
.lcsc-flow__error {
  min-height: 48px;
  padding: 12px;
  border: 1px solid var(--color-border);
  background: var(--color-surface-raised);
}

.lcsc-flow__error {
  display: grid;
  gap: 4px;
  border-color: color-mix(in srgb, var(--color-danger) 30%, var(--color-border));

  strong {
    color: var(--color-danger);
  }

  span {
    color: var(--color-muted);
    font-size: 13px;
    line-height: 1.5;
  }
}
</style>
