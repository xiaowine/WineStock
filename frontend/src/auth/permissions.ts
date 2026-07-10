// 本文件定义 frontend 使用的稳定权限代码和只读判断；它不替代后端实时授权检查。

/** 用户管理页面和操作使用的权限代码。 */
export const userPermissions = {
  register: 'user.register',
  read: 'user.read',
  updateStatus: 'user.status.update',
  delete: 'user.delete',
  updatePermissions: 'user.permissions.update',
  readPermissionDefinitions: 'user.permission.read',
  resetPassword: 'user.password.reset',
} as const

/** 判断权限快照是否包含指定权限。 */
export function hasPermission(
  permissions: readonly string[] | undefined,
  requiredPermission: string | undefined,
): boolean {
  return requiredPermission === undefined || permissions?.includes(requiredPermission) === true
}
