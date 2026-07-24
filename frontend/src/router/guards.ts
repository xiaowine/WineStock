// 本文件拥有 frontend 全局鉴权守卫、内部登录回跳、强制改密和会话失效导航；它不读取或持久化 token。
import { watch } from "vue";
import type { RouteLocationRaw, Router } from "vue-router";
import { hasPermission, hasPermissions } from "../auth/permissions";
import {
  authSession,
  authStatus,
  ensureAuthSessionInitialized,
  isLoggingOut,
} from "../auth/session";
import { runtimeSetupFinished } from "../shell/runtime";
import { getDefaultAppRouteName } from "./navigation";

let guardsInstalled = false;

/** 安装一次全局鉴权守卫，并监听停留期间的强制改密状态和会话失效。 */
export function installAuthGuards(router: Router): void {
  if (guardsInstalled) {
    return;
  }
  guardsInstalled = true;

  router.beforeEach(async (to) => {
    // 1. 运行设置等无服务依赖路由直接放行。
    if (to.meta.requiresService === false) {
      return;
    }
    // 2. Shell 尚未初始化配置或没有服务地址时先进入运行设置。
    if (!runtimeSetupFinished.value) {
      return createRuntimeSettingsRedirect(to.fullPath);
    }

    // 3. 会话初始化后再执行匿名/已认证与强制改密规则。
    const status = await ensureAuthSessionInitialized();
    if (to.meta.requiresAuth && status === "anonymous") {
      return createAuthEntryRedirect(to.fullPath);
    }
    if (
      status === "authenticated" &&
      (to.name === "auth-entry" || to.name === "register" || to.name === "login")
    ) {
      return {
        name: getDefaultAppRouteName(authSession.value?.user.permissions),
      };
    }
    if (
      status === "authenticated" &&
      authSession.value?.user.password_change_required &&
      !to.meta.allowsPasswordChangeRequired
    ) {
      const redirect = to.meta.requiresAuth
        ? to.fullPath
        : to.name === "login" && typeof to.query.redirect === "string"
          ? to.query.redirect
          : undefined;
      return createPasswordChangeRedirect(redirect);
    }
    if (
      status === "authenticated" &&
      !hasRoutePermissions(authSession.value?.user.permissions, to.meta)
    ) {
      const fallbackName = getDefaultAppRouteName(authSession.value?.user.permissions);
      if (fallbackName !== to.name) {
        return { name: fallbackName };
      }
    }
    if (to.name === "login" && status === "authenticated") {
      return {
        name: getDefaultAppRouteName(authSession.value?.user.permissions),
      };
    }
  });

  watch([authStatus, authSession], ([status, session]) => {
    if (isLoggingOut.value) {
      return;
    }

    const currentRoute = router.currentRoute.value;
    if (currentRoute.meta.requiresService === false) {
      return;
    }
    if (
      status === "authenticated" &&
      session?.user.password_change_required &&
      !currentRoute.meta.allowsPasswordChangeRequired
    ) {
      const redirect = currentRoute.meta.requiresAuth
        ? currentRoute.fullPath
        : currentRoute.name === "login" && typeof currentRoute.query.redirect === "string"
          ? currentRoute.query.redirect
          : undefined;
      void router.replace(createPasswordChangeRedirect(redirect)).catch((error: unknown) => {
        console.warn("强制改密状态恢复后无法跳转到修改密码页", error);
      });
      return;
    }

    if (
      status === "authenticated" &&
      !hasRoutePermissions(session?.user.permissions, currentRoute.meta)
    ) {
      const fallbackName = getDefaultAppRouteName(session?.user.permissions);
      if (fallbackName !== currentRoute.name) {
        void router.replace({ name: fallbackName }).catch((error: unknown) => {
          console.warn("当前会话权限变化后无法返回可用页面", error);
        });
      }
      return;
    }

    if (
      status !== "anonymous" ||
      !currentRoute.meta.requiresAuth ||
      currentRoute.name === "login"
    ) {
      return;
    }

    void router.replace(createAuthEntryRedirect(currentRoute.fullPath)).catch((error: unknown) => {
      console.warn("会话失效后无法跳转到登录页", error);
    });
  });
}

/** 同时兼容普通单权限页面和审批类复合权限页面。 */
function hasRoutePermissions(
  permissions: readonly string[] | undefined,
  meta: {
    requiredPermission?: string;
    requiredPermissions?: readonly string[];
  },
): boolean {
  return (
    hasPermissions(permissions, meta.requiredPermissions) &&
    hasPermission(permissions, meta.requiredPermission)
  );
}

/**
 * 解析登录后的内部回跳目标；拒绝外部、反斜杠和未匹配路由，失败时回到 dashboard。
 */
export function resolvePostLoginLocation(router: Router, redirect: unknown): RouteLocationRaw {
  const defaultRoute = {
    name: getDefaultAppRouteName(authSession.value?.user.permissions),
  };
  if (
    typeof redirect !== "string" ||
    !redirect.startsWith("/") ||
    redirect.startsWith("//") ||
    redirect.includes("\\")
  ) {
    return defaultRoute;
  }

  try {
    const resolved = router.resolve(redirect);
    if (resolved.matched.length === 0 || resolved.name === "home-fallback") {
      return defaultRoute;
    }
    return { path: resolved.fullPath };
  } catch {
    return defaultRoute;
  }
}

function createAuthEntryRedirect(fullPath: string): RouteLocationRaw {
  return {
    name: "auth-entry",
    query: { redirect: fullPath },
  };
}

function createPasswordChangeRedirect(fullPath: string | undefined): RouteLocationRaw {
  return {
    name: "change-password",
    query: fullPath ? { redirect: fullPath } : undefined,
  };
}

function createRuntimeSettingsRedirect(fullPath: string): RouteLocationRaw {
  return {
    name: "runtime-settings",
    query: fullPath === "/settings/runtime" ? undefined : { returnTo: fullPath },
  };
}
