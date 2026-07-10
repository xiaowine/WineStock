// 本文件拥有应用壳主导航配置和权限快照过滤，属于 frontend 路由层；后端仍决定最终授权。
import { hasPermission } from '../auth/permissions'

/** 应用壳导航分组；管理入口与高频业务入口分开呈现。 */
export type AppNavigationGroup = 'primary' | 'management'

/** 应用壳当前使用的线性图标名称。 */
export type AppNavigationIcon = 'dashboard' | 'items' | 'users'

/** 应用壳一级导航入口。 */
export interface AppNavigationItem {
  /** Vue Router 路由名称。 */
  routeName: string
  /** 面向用户的入口名称。 */
  label: string
  /** 侧栏信息分组。 */
  group: AppNavigationGroup
  /** 与入口语义对应的统一线性图标。 */
  icon: AppNavigationIcon
  /** 可选的页面读取权限；前端隐藏不替代服务端授权。 */
  requiredPermission?: string
}

/** 应用壳当前可见的一级导航入口。 */
export const appNavigation: readonly AppNavigationItem[] = [
  { routeName: 'dashboard', label: '总览', group: 'primary', icon: 'dashboard' },
  { routeName: 'items', label: '物品', group: 'primary', icon: 'items' },
  {
    routeName: 'users',
    label: '用户',
    group: 'management',
    icon: 'users',
    requiredPermission: 'user.read',
  },
]

/** 根据当前会话权限返回可见导航，不把前端隐藏当作安全边界。 */
export function getVisibleAppNavigation(permissions: readonly string[] | undefined) {
  return appNavigation.filter((item) =>
    hasPermission(permissions, item.requiredPermission),
  )
}
