// 本文件拥有应用壳主导航名称、分组和图标配置，属于 frontend 路由层；它不决定用户是否拥有业务权限。

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
}

/** 应用壳当前可见的一级导航入口。 */
export const appNavigation: readonly AppNavigationItem[] = [
  { routeName: 'dashboard', label: '总览', group: 'primary', icon: 'dashboard' },
  { routeName: 'items', label: '物品', group: 'primary', icon: 'items' },
]
