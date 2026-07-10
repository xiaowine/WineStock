// 本文件拥有应用壳主导航配置，属于 frontend 路由层；它不决定用户是否拥有业务权限。

/** 应用壳当前可见的一级导航入口。 */
export const appNavigation = [
  { routeName: 'dashboard', label: '总览' },
  { routeName: 'items', label: '物品' },
] as const
