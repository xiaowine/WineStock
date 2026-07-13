<!--
  本文件拥有用户名密码登录页面，属于 frontend 鉴权页面层。
  它在桌面和移动视口共用同一登录流程，不实现路由守卫或平台生命周期。
-->
<template>
  <main class="auth-page">
    <section class="auth-panel" aria-labelledby="login-title">
      <header class="auth-header">
        <div class="brand-lockup">
          <span class="brand-mark">W</span>
          <span class="brand-name">WineStock</span>
        </div>
        <div>
          <h1 id="login-title">{{ $route.meta.title }}</h1>
          <p>使用 WineStock 账号连接当前配置的库存服务。</p>
        </div>
      </header>

      <form class="auth-form" novalidate @submit.prevent="submitLogin">
        <div v-if="logoutWarning" class="form-warning" role="status">
          {{ logoutWarning }}
        </div>

        <FormInput
          v-model="username"
          label="用户名"
          validation-key="username"
          :error="usernameError"
          name="username"
          type="text"
          autocomplete="username"
          maxlength="64"
          :disabled="isSubmitting"
        />

        <FormField label="密码" control-id="login-password" validation-key="password" :error="passwordError" v-slot="{ describedBy, invalid }">
          <PasswordInput
            id="login-password"
            v-model="password"
            name="password"
            autocomplete="current-password"
            maxlength="256"
            :disabled="isSubmitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <div v-if="errorMessage" class="form-error" role="alert">
          {{ errorMessage }}
        </div>

        <button class="primary-button primary-button--full" type="submit" :disabled="isSubmitting">
          {{ isSubmitting ? '正在登录…' : '登录' }}
        </button>
      </form>

      <div class="auth-page-switch">
        <span>首次使用，当前服务还没有用户？</span>
        <RouterLink :to="{ name: 'register' }">创建首个用户</RouterLink>
      </div>

    </section>
  </main>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { login } from '../api/auth'
import {
  ApiConfigurationError,
  ApiError,
  ApiNetworkError,
  ApiResponseError,
} from '../api/errors'
import { resolveApiClientMetadata } from '../api/runtime-config'
import { establishAuthSession } from '../auth/session'
import { AuthPersistenceError } from '../auth/storage'
import { notice } from '../notices/notice'
import { resolvePostLoginLocation } from '../router/guards'
import PasswordInput from '../components/PasswordInput.vue'
import FormField from '../components/forms/FormField.vue'
import FormInput from '../components/forms/FormInput.vue'
import { useFormValidation } from '../composables/useFormValidation'

const router = useRouter()
const route = useRoute()
const username = ref('')
const password = ref('')
const isSubmitting = ref(false)
const errorMessage = ref('')
const fieldErrors = ref<Readonly<Record<string, readonly string[]>>>({})
useFormValidation(fieldErrors)

const usernameError = computed(() => fieldErrors.value.username?.[0])
const passwordError = computed(() => fieldErrors.value.password?.[0])
const logoutWarning = computed(() =>
  route.query.logout === 'local_only' ? '本机已退出，但服务端会话吊销未确认' : '',
)

/** 校验并提交登录表单；失败时保留输入并映射统一 API 错误。 */
async function submitLogin(): Promise<void> {
  errorMessage.value = ''
  fieldErrors.value = validateLoginInput(username.value, password.value)
  if (Object.keys(fieldErrors.value).length > 0) {
    return
  }

  isSubmitting.value = true
  try {
    const metadata = resolveApiClientMetadata()
    const response = await login({
      username: username.value.trim(),
      password: password.value,
      device_name: metadata.deviceName,
      client_kind: metadata.clientKind,
      version: metadata.appVersion,
    })

    establishAuthSession(response)
    await router.replace(resolvePostLoginLocation(router, route.query.redirect))
    notice.success('登录成功')
  } catch (error) {
    applyLoginError(error)
  } finally {
    isSubmitting.value = false
  }
}

function validateLoginInput(
  usernameValue: string,
  passwordValue: string,
): Readonly<Record<string, readonly string[]>> {
  const errors: Record<string, string[]> = {}
  if (!usernameValue.trim()) {
    errors.username = ['请输入用户名']
  }
  if (!passwordValue) {
    errors.password = ['请输入密码']
  }
  return errors
}

function applyLoginError(error: unknown): void {
  if (error instanceof AuthPersistenceError) {
    errorMessage.value = '登录成功，但无法保存登录状态，请检查浏览器存储权限'
    notice.error(errorMessage.value)
    return
  }
  if (error instanceof ApiError) {
    fieldErrors.value = error.fieldErrors
    errorMessage.value =
      error.code === 'invalid_credentials'
        ? '用户名或密码错误'
        : Object.keys(error.fieldErrors).length > 0
          ? '请检查输入内容'
          : error.message
    notice.error(errorMessage.value)
    return
  }
  if (error instanceof ApiConfigurationError) {
    errorMessage.value = error.message
    notice.error(errorMessage.value)
    return
  }
  if (error instanceof ApiNetworkError) {
    errorMessage.value = '无法连接到 WineStock 服务，请检查服务地址和运行状态'
    notice.error(errorMessage.value)
    return
  }
  if (error instanceof ApiResponseError) {
    errorMessage.value = '服务响应格式无效，请检查服务版本'
    notice.error(errorMessage.value)
    return
  }

  errorMessage.value = '登录失败，请稍后重试'
  notice.error(errorMessage.value)
}
</script>
