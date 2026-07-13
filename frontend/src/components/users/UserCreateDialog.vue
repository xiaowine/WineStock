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
      <FormInput
        v-model="username"
        label="用户名"
        validation-key="username"
        :error="fieldErrors.username"
        name="username"
        type="text"
        autocomplete="off"
        maxlength="64"
        autofocus
        :disabled="submitting"
      />

      <FormField label="初始密码" control-id="user-create-password" validation-key="password" :error="fieldErrors.password" hint="至少 8 个字符" v-slot="{ describedBy, invalid }">
        <PasswordInput
          id="user-create-password"
          v-model="password"
          name="password"
          autocomplete="new-password"
          minlength="8"
          maxlength="128"
          :disabled="submitting"
          :aria-invalid="invalid || undefined"
          :aria-describedby="describedBy"
        />
      </FormField>

      <FormField label="确认密码" control-id="user-create-password-confirmation" validation-key="confirmation" :error="fieldErrors.confirmation" v-slot="{ describedBy, invalid }">
        <PasswordInput
          id="user-create-password-confirmation"
          v-model="confirmation"
          name="password_confirmation"
          autocomplete="new-password"
          maxlength="128"
          :disabled="submitting"
          :aria-invalid="invalid || undefined"
          :aria-describedby="describedBy"
        />
      </FormField>

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
import PasswordInput from '../PasswordInput.vue'
import FormField from '../forms/FormField.vue'
import FormInput from '../forms/FormInput.vue'
import { useFormValidation } from '../../composables/useFormValidation'

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
useFormValidation(fieldErrors)

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
