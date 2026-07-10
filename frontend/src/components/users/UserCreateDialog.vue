<!--
  本文件拥有创建后续用户的输入和本地校验，属于 frontend 用户管理组件层。
  它不调用注册 API，也不分配新用户权限。
-->
<template>
  <ModalDialog
    :open="open"
    title="创建用户"
    description="新用户创建后默认没有权限，可在用户列表中继续分配。"
    :busy="submitting"
    @close="emit('close')"
  >
    <form id="user-create-form" class="dialog-form" novalidate @submit.prevent="submit">
      <label class="form-field">
        <span>用户名</span>
        <input
          v-model="username"
          name="username"
          type="text"
          autocomplete="off"
          maxlength="64"
          autofocus
          :disabled="submitting"
          :aria-invalid="Boolean(fieldErrors.username)"
        />
        <small v-if="fieldErrors.username" class="field-error">{{ fieldErrors.username }}</small>
      </label>

      <label class="form-field">
        <span>初始密码</span>
        <input
          v-model="password"
          name="password"
          type="password"
          autocomplete="new-password"
          minlength="8"
          maxlength="128"
          :disabled="submitting"
          :aria-invalid="Boolean(fieldErrors.password)"
        />
        <small class="field-hint">至少 8 个字符</small>
        <small v-if="fieldErrors.password" class="field-error">{{ fieldErrors.password }}</small>
      </label>

      <label class="form-field">
        <span>确认密码</span>
        <input
          v-model="confirmation"
          name="password_confirmation"
          type="password"
          autocomplete="new-password"
          maxlength="128"
          :disabled="submitting"
          :aria-invalid="Boolean(fieldErrors.confirmation)"
        />
        <small v-if="fieldErrors.confirmation" class="field-error">
          {{ fieldErrors.confirmation }}
        </small>
      </label>

      <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
    </form>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        取消
      </button>
      <button
        class="primary-button"
        type="submit"
        form="user-create-form"
        :disabled="submitting"
      >
        {{ submitting ? '正在创建…' : '创建用户' }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import ModalDialog from '../ModalDialog.vue'

const props = defineProps<{
  open: boolean
  submitting: boolean
  errorMessage: string
}>()

const emit = defineEmits<{
  close: []
  submit: [request: { username: string; password: string }]
}>()

const username = ref('')
const password = ref('')
const confirmation = ref('')
const fieldErrors = ref<Record<string, string>>({})

watch(
  () => props.open,
  (open) => {
    if (open) {
      username.value = ''
      password.value = ''
      confirmation.value = ''
      fieldErrors.value = {}
    }
  },
)

function submit(): void {
  const errors: Record<string, string> = {}
  const normalizedUsername = username.value.trim()
  if (!normalizedUsername) {
    errors.username = '请输入用户名'
  }
  if (!password.value) {
    errors.password = '请输入初始密码'
  } else if (password.value.length < 8) {
    errors.password = '密码至少需要 8 个字符'
  }
  if (!confirmation.value) {
    errors.confirmation = '请再次输入密码'
  } else if (confirmation.value !== password.value) {
    errors.confirmation = '两次输入的密码不一致'
  }
  fieldErrors.value = errors
  if (Object.keys(errors).length === 0) {
    emit('submit', { username: normalizedUsername, password: password.value })
  }
}
</script>
