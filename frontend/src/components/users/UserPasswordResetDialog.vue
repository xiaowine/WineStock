<!--
  本文件拥有管理员设置临时密码的输入和本地校验，属于 frontend 用户管理组件层。
  它不调用密码重置 API，也不持久化任何密码。
-->
<template>
  <ModalDialog
    :open="Boolean(user)"
    title="设置临时密码"
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
        设置后，该用户将在所有已登录设备上退出；下次登录时必须修改密码。
      </p>

      <form id="user-password-reset-form" class="dialog-form" novalidate @submit.prevent="submit">
        <FormField label="临时密码" control-id="user-temporary-password" validation-key="password" :error="fieldErrors.password" v-slot="{ describedBy, invalid }">
          <PasswordInput
            id="user-temporary-password"
            v-model="password"
            name="temporary_password"
            autocomplete="new-password"
            minlength="8"
            maxlength="128"
            autofocus
            :disabled="submitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <FormField label="确认临时密码" control-id="user-temporary-password-confirmation" validation-key="confirmation" :error="fieldErrors.confirmation" v-slot="{ describedBy, invalid }">
          <PasswordInput
            id="user-temporary-password-confirmation"
            v-model="confirmation"
            name="temporary_password_confirmation"
            autocomplete="new-password"
            maxlength="128"
            :disabled="submitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
      </form>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        取消
      </button>
      <button
        class="primary-button"
        type="submit"
        form="user-password-reset-form"
        :disabled="submitting"
      >
        {{ submitting ? '正在设置…' : '设置临时密码' }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import type { UserAdminResponse } from '../../api/users'
import ModalDialog from '../ModalDialog.vue'
import PasswordInput from '../PasswordInput.vue'
import FormField from '../forms/FormField.vue'
import { useFormValidation } from '../../composables/useFormValidation'

const props = defineProps<{
  user: UserAdminResponse | null
  submitting: boolean
  errorMessage: string
}>()

const emit = defineEmits<{
  close: []
  submit: [password: string]
}>()

const password = ref('')
const confirmation = ref('')
const fieldErrors = ref<Record<string, string>>({})
useFormValidation(fieldErrors)

watch(
  () => props.user,
  (user) => {
    if (user) {
      password.value = ''
      confirmation.value = ''
      fieldErrors.value = {}
    }
  },
)

function submit(): void {
  const errors: Record<string, string> = {}
  if (!password.value) {
    errors.password = '请输入临时密码'
  } else if (password.value.length < 8) {
    errors.password = '密码至少需要 8 个字符'
  }
  if (!confirmation.value) {
    errors.confirmation = '请再次输入临时密码'
  } else if (confirmation.value !== password.value) {
    errors.confirmation = '两次输入的密码不一致'
  }
  fieldErrors.value = errors
  if (Object.keys(errors).length === 0) {
    emit('submit', password.value)
  }
}
</script>
