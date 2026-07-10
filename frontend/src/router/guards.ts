// 本文件拥有 frontend 全局鉴权守卫、内部登录回跳和会话失效导航；它不读取或持久化 token。
import { watch } from 'vue'
import type { RouteLocationRaw, Router } from 'vue-router'
import {
  authStatus,
  ensureAuthSessionInitialized,
  isLoggingOut,
} from '../auth/session'

let guardsInstalled = false

/** 安装一次全局鉴权守卫和停留页面期间的会话失效监听。 */
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
    if (to.name === 'login' && status === 'authenticated') {
      return { name: 'dashboard' }
    }
  })

  watch(authStatus, (status) => {
    if (status !== 'anonymous' || isLoggingOut.value) {
      return
    }

    const currentRoute = router.currentRoute.value
    if (!currentRoute.meta.requiresAuth || currentRoute.name === 'login') {
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
    if (resolved.matched.length === 0 || resolved.name === 'not-found') {
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
