// 本文件拥有桌面与移动应用壳共用的退出编排；它不保存 token，也不实现平台专用会话清理。
import { ref } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { AuthPersistenceError } from "../auth/storage";
import { isLoggingOut, logoutAuthSession, type LogoutResult } from "../auth/session";
import { notice } from "../notices/notice";

/**
 * 为应用壳提供统一退出行为。
 * 调用会吊销或清除当前会话并跳转统一认证入口；失败时保留可展示错误，服务端吊销未确认时仍完成本机退出。
 */
export function useShellLogout() {
  const router = useRouter();
  const { t } = useI18n();
  const logoutError = ref("");

  async function handleLogout(): Promise<void> {
    logoutError.value = "";

    let result: LogoutResult;
    try {
      result = await logoutAuthSession();
    } catch (error) {
      logoutError.value =
        error instanceof AuthPersistenceError
          ? t("auth.logoutPersistenceFailed")
          : t("auth.logoutFailed");
      notice.error(logoutError.value);
      return;
    }

    // 与守卫、运行设置出口一致：匿名统一进 /auth，query 透传到 login/register。
    await router.replace({
      name: "auth-entry",
      query: result === "local_only" ? { logout: "local_only" } : undefined,
    });
    if (result === "local_only") {
      notice.warning(t("auth.logoutLocalOnlyWarning"));
    } else {
      notice.success(t("auth.loggedOut"));
    }
  }

  return {
    handleLogout,
    isLoggingOut,
    logoutError,
  };
}
