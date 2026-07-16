<!--
  本文件拥有用户软删除确认内容，属于 frontend 用户管理组件层。
  它不调用删除 API，也不替代后端的操作者保护和防锁死校验。
-->
<template>
  <ModalDialog :open="Boolean(user)" title="删除用户" :busy="submitting" @close="emit('close')">
    <template #context>
      <div v-if="user" class="dialog-account-context dialog-account-context--danger">
        <span>目标用户</span>
        <strong :title="user.username">{{ user.username }}</strong>
      </div>
    </template>

    <div class="dialog-content">
      <p class="confirmation-copy">
        删除后，该账号会立即退出所有设备，并且无法再登录或出现在用户列表中。
      </p>
      <p class="form-warning">此操作无法撤销，用户名也不能重新注册使用。历史业务记录仍会保留。</p>
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
import type { UserAdminResponse } from "../../api/users";
import ModalDialog from "../ModalDialog.vue";

defineProps<{
  user: UserAdminResponse | null;
  submitting: boolean;
  errorMessage: string;
}>();

const emit = defineEmits<{
  close: [];
  submit: [];
}>();
</script>
