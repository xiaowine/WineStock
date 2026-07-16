// 本文件从应用路由目录生成主导航并按权限过滤，属于 frontend 路由层；它不重复声明页面名称或权限。
import { hasPermission, hasPermissions } from '../auth/permissions'
import {
  appRouteCatalog,
  type AppNavigationGroup,
  type AppNavigationIcon,
  type AppRouteName,
} from './appRouteCatalog'

export type { AppNavigationGroup, AppNavigationIcon } from './appRouteCatalog'

/** 应用壳一级导航入口。 */
export interface AppNavigationItem {
  /** Vue Router 路由名称。 */
  routeName: AppRouteName
  /** 面向用户的入口名称。 */
  label: string
  /** 侧栏信息分组。 */
  group: AppNavigationGroup
  /** 与入口语义对应的统一线性图标。 */
  icon: AppNavigationIcon
  /** 可选的页面读取权限；前端隐藏不替代服务端授权。 */
  requiredPermission?: string
  /** 页面入口必须同时满足的权限组合。 */
  requiredPermissions?: readonly string[]
  /** 是否只在桌面应用壳中展示。 */
  desktopOnly?: boolean
}

/** 应用壳当前可见的一级导航入口。 */
export const appNavigation: readonly AppNavigationItem[] = (
  Object.keys(appRouteCatalog) as AppRouteName[]
)
  .sort((left, right) => {
    const leftNavigation = appRouteCatalog[left].navigation
    const rightNavigation = appRouteCatalog[right].navigation
    if (leftNavigation.group !== rightNavigation.group) {
      return leftNavigation.group === 'primary' ? -1 : 1
    }
    return leftNavigation.order - rightNavigation.order
  })
  .map((routeName) => {
    const entry = appRouteCatalog[routeName]
    return {
      routeName,
      label: entry.title,
      group: entry.navigation.group,
      icon: entry.navigation.icon,
      requiredPermission: entry.requiredPermission,
      requiredPermissions:
        'requiredPermissions' in entry ? entry.requiredPermissions : [entry.requiredPermission],
      desktopOnly: 'desktopOnly' in entry.navigation ? entry.navigation.desktopOnly : undefined,
    }
  })

/** 根据当前会话权限返回可见导航，不把前端隐藏当作安全边界。 */
export function getVisibleAppNavigation(
  permissions: readonly string[] | undefined,
  options: { includeDesktopOnly?: boolean } = { includeDesktopOnly: true },
) {
  return appNavigation.filter(
    (item) =>
      hasPermission(permissions, item.requiredPermission) &&
      hasPermissions(permissions, item.requiredPermissions) &&
      (options.includeDesktopOnly !== false || item.desktopOnly !== true),
  )
}

/** 返回当前权限下第一个可用业务入口；无可见入口时保留库存总览作为错误承载页。 */
export function getDefaultAppRouteName(permissions: readonly string[] | undefined): string {
  return getVisibleAppNavigation(permissions)[0]?.routeName ?? 'dashboard'
}
