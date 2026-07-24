// 本文件拥有运行设置离开目标与 returnTo→redirect 桥接的纯规则；它不调用路由或 Shell Bridge。

const RUNTIME_SETTINGS_PATH = "/settings/runtime";
const AUTH_PATHS = new Set(["/auth", "/login", "/register", "/change-password"]);

/** 运行设置页离开后的导航目标。 */
export type RuntimeSettingsLeaveTarget =
  | { kind: "stay" }
  | { kind: "auth"; redirect?: string }
  | { kind: "path"; path: string }
  | { kind: "default-app" };

/** 拒绝外部、协议相对和反斜杠路径。 */
export function isSafeInternalPath(path: string): boolean {
  return path.startsWith("/") && !path.startsWith("//") && !path.includes("\\");
}

/**
 * 将运行设置的 returnTo 规范为认证入口的 redirect。
 * 业务路径原样返回；认证相关路径提取嵌套 redirect；设置页或非法路径丢弃。
 */
export function bridgeReturnToToAuthRedirect(returnTo: string | undefined): string | undefined {
  if (!returnTo || !isSafeInternalPath(returnTo)) {
    return undefined;
  }

  const { pathname, searchParams } = splitPathAndQuery(returnTo);
  if (pathname === RUNTIME_SETTINGS_PATH) {
    return undefined;
  }

  if (AUTH_PATHS.has(pathname)) {
    const nested = searchParams.get("redirect");
    if (!nested || !isSafeInternalPath(nested)) {
      return undefined;
    }
    const nestedPath = splitPathAndQuery(nested).pathname;
    if (nestedPath === RUNTIME_SETTINGS_PATH || AUTH_PATHS.has(nestedPath)) {
      return undefined;
    }
    return nested;
  }

  return returnTo;
}

/**
 * 根据设置是否完成、会话与 returnTo 解析运行设置离开目标。
 * `setupFinished` 为 false（Shell 尚未 initialized 或服务不可用）时匿名不得离开。
 * `returnToRouteValid` 由调用方用路由器 resolve 结果填入（已排除 runtime-settings / home-fallback）。
 */
export function resolveRuntimeSettingsLeave(input: {
  returnTo: string | undefined;
  /** 设置流程已完成（Shell initialized 且服务可访问）。 */
  setupFinished: boolean;
  authenticated: boolean;
  returnToRouteValid: boolean;
}): RuntimeSettingsLeaveTarget {
  const { returnTo, setupFinished, authenticated, returnToRouteValid } = input;

  if (returnTo && returnToRouteValid) {
    if (authenticated) {
      return { kind: "path", path: returnTo };
    }
    if (!setupFinished) {
      return { kind: "stay" };
    }
    const redirect = bridgeReturnToToAuthRedirect(returnTo);
    return redirect ? { kind: "auth", redirect } : { kind: "auth" };
  }

  if (authenticated) {
    return { kind: "default-app" };
  }
  if (setupFinished) {
    return { kind: "auth" };
  }
  return { kind: "stay" };
}

function splitPathAndQuery(fullPath: string): {
  pathname: string;
  searchParams: URLSearchParams;
} {
  const queryIndex = fullPath.indexOf("?");
  if (queryIndex === -1) {
    return { pathname: fullPath, searchParams: new URLSearchParams() };
  }
  return {
    pathname: fullPath.slice(0, queryIndex),
    searchParams: new URLSearchParams(fullPath.slice(queryIndex + 1)),
  };
}
