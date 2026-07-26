<!-- 本组件拥有单个立创商品编号查询、错误恢复和应用确认；它不直接修改物品草稿。 -->
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
      v-if="!candidate"
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
        :disabled="pending"
        :error="inputError"
        :title="inputError || undefined"
        @update:model-value="inputError = ''"
      />

      <div v-if="pending" class="lcsc-lookup__status" role="status">正在查询立创资料…</div>
      <div v-else-if="lookupError" class="lcsc-lookup__error" role="alert">
        <strong>查询失败</strong>
        <span>{{ lookupError }}</span>
      </div>
    </form>

    <section v-else class="lcsc-lookup-result" aria-live="polite">
      <strong>{{ candidate.name }}</strong>
      <span>
        立创商品编号 {{ candidate.product_code
        }}<template v-if="candidate.manufacturer"> · {{ candidate.manufacturer }}</template>
      </span>
      <p>是否使用查询结果填写当前表单？查询结果中的有效字段将覆盖当前内容。</p>
      <FormSelect
        :id="`${formId}-template`"
        v-model="selectedTemplateId"
        class="lcsc-lookup-result__template"
        label="属性模板"
        match-trigger-width
        name="lcsc_template_id"
      >
        <option :value="null">不使用模板</option>
        <option v-for="template in templates" :key="template.id" :value="template.id">
          {{ template.name }}
        </option>
      </FormSelect>
    </section>

    <template #actions>
      <template v-if="candidate">
        <button class="secondary-button" type="button" @click="requestClose">不填写</button>
        <button class="primary-button" type="button" @click="applyCandidate">覆盖填写</button>
      </template>
      <template v-else>
        <button class="secondary-button" type="button" :disabled="pending" @click="requestClose">
          取消
        </button>
        <button class="primary-button" type="submit" :form="formId" :disabled="pending">
          {{ pending ? "正在查询…" : lookupError ? "重新查询" : "查询" }}
        </button>
      </template>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, useId, watch } from "vue";
import { ApiError, ApiNetworkError } from "../../api/errors";
import type { ItemAttributeTemplateResponse } from "../../api/itemAttributeTemplates";
import { lookupLcscItem, type LcscItemLookupResponse } from "../../api/items";
import FormInput from "../forms/FormInput.vue";
import FormSelect from "../forms/FormSelect.vue";
import ModalDialog from "../ModalDialog.vue";

const props = withDefaults(
  defineProps<{
    open: boolean;
    initialCode?: string;
    /** 打开时若 initialCode 是合法 C 号则立即查询；用于扫码入口跳过手动提交。 */
    autoQuery?: boolean;
    templates: ItemAttributeTemplateResponse[];
  }>(),
  { initialCode: "", autoQuery: false },
);

const emit = defineEmits<{
  close: [];
  apply: [candidate: LcscItemLookupResponse, templateId: number | null];
}>();

const formId = `lcsc-lookup-${useId()}`;
const productCode = ref("");
const inputError = ref("");
const lookupError = ref("");
const pending = ref(false);
const candidate = ref<LcscItemLookupResponse | null>(null);
const selectedTemplateId = ref<number | null>(null);
let requestController: AbortController | null = null;
let requestGeneration = 0;

watch(
  () => props.open,
  async (open) => {
    abortRequest();
    if (!open) return;
    productCode.value = normalizeCode(props.initialCode);
    inputError.value = "";
    lookupError.value = "";
    candidate.value = null;
    selectedTemplateId.value = props.templates[0]?.id ?? null;
    if (props.autoQuery && /^C[0-9]+$/.test(productCode.value)) {
      void submitLookup();
      return;
    }
    await nextTick();
    focusCodeInput();
  },
);

onBeforeUnmount(abortRequest);

async function submitLookup(): Promise<void> {
  const normalized = normalizeCode(productCode.value);
  productCode.value = normalized;
  if (!/^C[0-9]+$/.test(normalized)) {
    inputError.value = "商品编号必须以 C 开头，后续全部为数字。";
    focusCodeInput();
    return;
  }

  abortRequest();
  const controller = new AbortController();
  const generation = ++requestGeneration;
  requestController = controller;
  pending.value = true;
  lookupError.value = "";
  try {
    const result = await lookupLcscItem(normalized, controller.signal);
    if (generation !== requestGeneration) return;
    candidate.value = result;
    selectedTemplateId.value = props.templates[0]?.id ?? null;
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") return;
    if (generation === requestGeneration) lookupError.value = lookupErrorMessage(error);
  } finally {
    if (requestController === controller) {
      requestController = null;
      pending.value = false;
    }
  }
}

function requestClose(): void {
  abortRequest();
  emit("close");
}

function applyCandidate(): void {
  if (!candidate.value) return;
  emit("apply", candidate.value, selectedTemplateId.value);
  emit("close");
}

function abortRequest(): void {
  requestGeneration += 1;
  requestController?.abort();
  requestController = null;
  pending.value = false;
}

function normalizeCode(value: string): string {
  return value.trim().toUpperCase();
}

function focusCodeInput(): void {
  document.getElementById(`${formId}-product-code`)?.focus();
}

function lookupErrorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    const messages: Record<string, string> = {
      invalid_lcsc_product_code: "商品编号格式无效，请输入 C 开头、后续为数字的编号。",
      lcsc_product_not_found: "没有查询到该立创商品，请检查编号。",
      lcsc_lookup_busy: "查询服务繁忙，请稍后重试。",
      lcsc_lookup_timeout: "立创服务响应超时，请稍后重试。",
      lcsc_lookup_failed: "暂时无法连接立创资料服务。",
      lcsc_invalid_response: "立创返回了无法识别的数据。",
    };
    return messages[error.code] ?? error.message;
  }
  if (error instanceof ApiNetworkError) return "无法连接 WineStock 服务。";
  return "查询过程中发生未知错误，请稍后重试。";
}
</script>

<style lang="scss" src="./LcscItemLookupDialog.scss"></style>
