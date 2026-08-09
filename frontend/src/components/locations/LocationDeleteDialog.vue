<!--
  本组件拥有库位分组或库位软删除的确认内容。
  它不调用删除 API，也不提供强制或级联删除行为。
-->
<template>
  <ModalDialog
    :open="Boolean(target)"
    :title="target?.kind === 'group' ? $t('locations.deleteGroupTitle') : $t('locations.deleteLocation')"
    :busy="submitting"
    @close="emit('close')"
  >
    <template #context>
      <div v-if="target" class="dialog-account-context dialog-account-context--danger">
        <span>{{
          target.kind === "group" ? $t('locations.targetGroup') : $t('locations.targetLocation')
        }}</span>
        <strong :title="target.label">{{ target.label }}</strong>
      </div>
    </template>

    <div class="dialog-content">
      <p class="confirmation-copy">
        {{
          target?.kind === "group"
            ? $t('locations.deleteGroupConfirmation')
            : $t('locations.deleteLocationConfirmation')
        }}
      </p>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        {{ $t('locations.cancel') }}
      </button>
      <button class="danger-button" type="button" :disabled="submitting" @click="emit('submit')">
        {{ submitting ? $t('locations.deleting') : $t('locations.confirmDelete') }}
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
