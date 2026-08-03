// 用户目录展示层的纯函数；不请求 API，也不承担后端授权。
import type { UserAdminResponse, UserStatus } from "../../api/users";

export interface UserDirectoryCapabilities {
  canUpdateUsername: boolean;
  canEditPermissions: boolean;
  canResetPassword: boolean;
  canUpdateStatus: boolean;
  canDelete: boolean;
}

export function isCurrentUser(user: UserAdminResponse, currentUserId?: string): boolean {
  return currentUserId === String(user.id);
}

export function hasAvailableUserAction(
  user: UserAdminResponse,
  currentUserId: string | undefined,
  capabilities: UserDirectoryCapabilities,
): boolean {
  return (
    capabilities.canUpdateUsername ||
    capabilities.canEditPermissions ||
    (!isCurrentUser(user, currentUserId) &&
      (capabilities.canResetPassword || capabilities.canUpdateStatus || capabilities.canDelete))
  );
}

export function userStatusLabel(status: UserStatus): string {
  return status === "active" ? "已启用" : "已停用";
}

export function userStatusClass(status: UserStatus): string {
  return status === "active" ? "status-pill--ok" : "status-pill--neutral";
}

export function formatUserDate(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat("zh-CN", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      }).format(date);
}
