<!--
  本文件拥有用户启停确认内容，属于 frontend 用户管理组件层。
  它不调用状态更新 API，也不决定防锁死规则。
-->
<template>
  <ModalDialog
    :open="Boolean(user)"
    :title="nextStatus === 'disabled' ? '停用用户' : '启用用户'"
    :busy="submitting"
    @close="emit('close')"
  >
    <template #context>
      <div v-if="user" class="dialog-account-context">
        <span>目标用户</span>
        <strong :title="user.username">{{ user.username }}</strong>
      </div>
    </template>

    <div class="dialog-content">
      <p class="confirmation-copy">
        {{
          nextStatus === 'disabled'
            ? '停用后，该用户将在所有已登录设备上退出，并且无法再次登录，直到账号重新启用。'
            : '启用后，该用户可以重新登录。'
        }}
      </p>
      <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        取消
      </button>
      <button
        class="primary-button"
        type="button"
        :disabled="submitting"
        @click="emit('submit')"
      >
        {{ submitting ? '正在保存…' : nextStatus === 'disabled' ? '确认停用' : '确认启用' }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import type { UserAdminResponse, UserStatus } from '../../api/users'
import ModalDialog from '../ModalDialog.vue'

defineProps<{
  user: UserAdminResponse | null
  nextStatus: UserStatus
  submitting: boolean
  errorMessage: string
}>()

const emit = defineEmits<{
  close: []
  submit: []
}>()

</script>
