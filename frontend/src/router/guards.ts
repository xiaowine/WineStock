// 本文件拥有 frontend 全局鉴权守卫、内部登录回跳、强制改密和会话失效导航；它不读取或持久化 token。
import { watch } from 'vue'
import type { RouteLocationRaw, Router } from 'vue-router'
import { hasPermission } from '../auth/permissions'
import {
  authSession,
  authStatus,
  ensureAuthSessionInitialized,
  isLoggingOut,
} from '../auth/session'

let guardsInstalled = false

/** 安装一次全局鉴权守卫，并监听停留期间的强制改密状态和会话失效。 */
export function installAuthGuards(router: Router): void {
  if (guardsInstalled) {
    return
  }
  guardsInstalled = true

  router.beforeEach(async (to) => {
    const status = await ensureAuthSessionInitialized()
    if (to.meta.requiresAuth && status === 'anonymous') {
      return createLoginRedirect(to.fullPath)
    }
    if (
      status === 'authenticated' &&
      authSession.value?.user.password_change_required &&
      !to.meta.allowsPasswordChangeRequired
    ) {
      const redirect =
        to.meta.requiresAuth
          ? to.fullPath
          : to.name === 'login' && typeof to.query.redirect === 'string'
            ? to.query.redirect
            : undefined
      return createPasswordChangeRedirect(redirect)
    }
    if (
      status === 'authenticated' &&
      !hasPermission(authSession.value?.user.permissions, to.meta.requiredPermission)
    ) {
      return { name: 'dashboard' }
    }
    if (to.name === 'login' && status === 'authenticated') {
      return { name: 'dashboard' }
    }
  })

  watch([authStatus, authSession], ([status, session]) => {
    if (isLoggingOut.value) {
      return
    }

    const currentRoute = router.currentRoute.value
    if (
      status === 'authenticated' &&
      session?.user.password_change_required &&
      !currentRoute.meta.allowsPasswordChangeRequired
    ) {
      const redirect =
        currentRoute.meta.requiresAuth
          ? currentRoute.fullPath
          : currentRoute.name === 'login' && typeof currentRoute.query.redirect === 'string'
            ? currentRoute.query.redirect
            : undefined
      void router
        .replace(createPasswordChangeRedirect(redirect))
        .catch((error: unknown) => {
          console.warn('强制改密状态恢复后无法跳转到修改密码页', error)
        })
      return
    }

    if (
      status === 'authenticated' &&
      !hasPermission(session?.user.permissions, currentRoute.meta.requiredPermission)
    ) {
      void router.replace({ name: 'dashboard' }).catch((error: unknown) => {
        console.warn('当前会话权限变化后无法返回总览', error)
      })
      return
    }

    if (
      status !== 'anonymous' ||
      !currentRoute.meta.requiresAuth ||
      currentRoute.name === 'login'
    ) {
      return
    }

    void router.replace(createLoginRedirect(currentRoute.fullPath)).catch((error: unknown) => {
      console.warn('会话失效后无法跳转到登录页', error)
    })
  })
}

/**
 * 解析登录后的内部回跳目标；拒绝外部、反斜杠和未匹配路由，失败时回到 dashboard。
 */
export function resolvePostLoginLocation(router: Router, redirect: unknown): RouteLocationRaw {
  if (
    typeof redirect !== 'string' ||
    !redirect.startsWith('/') ||
    redirect.startsWith('//') ||
    redirect.includes('\\')
  ) {
    return { name: 'dashboard' }
  }

  try {
    const resolved = router.resolve(redirect)
    if (resolved.matched.length === 0 || resolved.name === 'home-fallback') {
      return { name: 'dashboard' }
    }
    return { path: resolved.fullPath }
  } catch {
    return { name: 'dashboard' }
  }
}

function createLoginRedirect(fullPath: string): RouteLocationRaw {
  return {
    name: 'login',
    query: { redirect: fullPath },
  }
}

function createPasswordChangeRedirect(fullPath: string | undefined): RouteLocationRaw {
  return {
    name: 'change-password',
    query: fullPath ? { redirect: fullPath } : undefined,
  }
}
