<!--
  本文件拥有桌面端用户名密码登录页面，属于 frontend 鉴权页面层。
  它调用 auth API 并建立可恢复会话，不实现路由守卫或平台生命周期。
-->
<template>
  <main class="desktop-auth-page">
    <section class="desktop-auth-panel" aria-labelledby="desktop-login-title">
      <header class="desktop-auth-header">
        <div class="brand-lockup">
          <span class="brand-mark">W</span>
          <span class="brand-name">WineStock</span>
        </div>
        <div>
          <p class="eyebrow">Authentication</p>
          <h1 id="desktop-login-title">登录</h1>
          <p>使用 WineStock 账号连接当前配置的库存服务。</p>
        </div>
      </header>

      <form class="auth-form" novalidate @submit.prevent="submitLogin">
        <div v-if="logoutWarning" class="form-warning" role="status">
          {{ logoutWarning }}
        </div>

        <label class="form-field">
          <span>用户名</span>
          <input
            v-model="username"
            name="username"
            type="text"
            autocomplete="username"
            maxlength="64"
            :disabled="isSubmitting"
            :aria-invalid="Boolean(usernameError)"
            :aria-describedby="usernameError ? 'login-username-error' : undefined"
          />
          <small v-if="usernameError" id="login-username-error" class="field-error">
            {{ usernameError }}
          </small>
        </label>

        <label class="form-field">
          <span>密码</span>
          <input
            v-model="password"
            name="password"
            type="password"
            autocomplete="current-password"
            maxlength="256"
            :disabled="isSubmitting"
            :aria-invalid="Boolean(passwordError)"
            :aria-describedby="passwordError ? 'login-password-error' : undefined"
          />
          <small v-if="passwordError" id="login-password-error" class="field-error">
            {{ passwordError }}
          </small>
        </label>

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

      <p class="auth-runtime-note">
        服务地址由平台运行时配置或 <code>VITE_API_BASE_URL</code> 提供。
      </p>
    </section>
  </main>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { login } from '../../api/auth'
import {
  ApiConfigurationError,
  ApiError,
  ApiNetworkError,
  ApiResponseError,
} from '../../api/errors'
import { resolveApiClientMetadata } from '../../api/runtime-config'
import { establishAuthSession } from '../../auth/session'
import { AuthPersistenceError } from '../../auth/storage'
import { resolvePostLoginLocation } from '../../router/guards'

const router = useRouter()
const route = useRoute()
const username = ref('')
const password = ref('')
const isSubmitting = ref(false)
const errorMessage = ref('')
const fieldErrors = ref<Readonly<Record<string, readonly string[]>>>({})

const usernameError = computed(() => fieldErrors.value.username?.[0])
const passwordError = computed(() => fieldErrors.value.password?.[0])
const logoutWarning = computed(() =>
  route.query.logout === 'local_only' ? '本机已退出，但服务端会话吊销未确认' : '',
)

/** 校验并提交桌面登录表单；失败时保留输入并映射统一 API 错误。 */
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

    if (response.user.password_change_required) {
      errorMessage.value = '该账号需要修改临时密码，但当前前端尚未启用此流程'
      return
    }

    establishAuthSession(response)
    await router.replace(resolvePostLoginLocation(router, route.query.redirect))
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
    return
  }
  if (error instanceof ApiConfigurationError) {
    errorMessage.value = error.message
    return
  }
  if (error instanceof ApiNetworkError) {
    errorMessage.value = '无法连接到 WineStock 服务，请检查服务地址和运行状态'
    return
  }
  if (error instanceof ApiResponseError) {
    errorMessage.value = '服务响应格式无效，请检查服务版本'
    return
  }

  errorMessage.value = '登录失败，请稍后重试'
}
</script>
