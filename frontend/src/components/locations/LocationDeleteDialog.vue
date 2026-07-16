<!--
  本组件拥有库位分组或库位软删除的确认内容。
  它不调用删除 API，也不提供强制或级联删除行为。
-->
<template>
  <ModalDialog
    :open="Boolean(target)"
    :title="target?.kind === 'group' ? '删除库位分组' : '删除库位'"
    :busy="submitting"
    @close="emit('close')"
  >
    <template #context>
      <div v-if="target" class="dialog-account-context dialog-account-context--danger">
        <span>{{ target.kind === "group" ? "目标分组" : "目标库位" }}</span>
        <strong :title="target.label">{{ target.label }}</strong>
      </div>
    </template>

    <div class="dialog-content">
      <p class="confirmation-copy">
        {{
          target?.kind === "group"
            ? "只有不包含子分组和有效库位的空分组才能删除。"
            : "只有没有当前库存批次引用的库位才能删除，历史单据不会被移除。"
        }}
      </p>
      <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        取消
      </button>
      <button class="danger-button" type="button" :disabled="submitting" @click="emit('submit')">
        {{ submitting ? "正在删除…" : "确认删除" }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import ModalDialog from "../ModalDialog.vue";
import type { LocationDeleteTarget } from "./types";

defineProps<{
  target: LocationDeleteTarget | null;
  submitting: boolean;
  errorMessage: string;
}>();

const emit = defineEmits<{
  close: [];
  submit: [];
}>();
</script>
