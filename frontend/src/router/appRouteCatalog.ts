// 本文件集中声明应用壳一级路由的名称、权限和导航呈现，属于 frontend 路由契约层；它不创建 Router 或执行权限判断。
import type { RouteMeta } from "vue-router";
import { stockPermissions, userPermissions } from "../auth/permissions";

/** 应用壳导航分组；高频业务入口与管理入口分开呈现。 */
export type AppNavigationGroup = "primary" | "management";

/** 应用壳导航使用的语义化线性图标名称。 */
export type AppNavigationIcon =
  | "dashboard"
  | "items"
  | "inbound-create"
  | "inbound-orders"
  | "outbound-create"
  | "outbound-orders"
  | "inbound-approvals"
  | "outbound-approvals"
  | "locations"
  | "templates"
  | "substitutes"
  | "events"
  | "users";

/** 单个应用路由在侧栏或移动导航中的呈现规则。 */
export interface AppRouteNavigation {
  /** 导航所属的信息分组。 */
  group: AppNavigationGroup;
  /** 与入口业务语义对应的线性图标。 */
  icon: AppNavigationIcon;
  /** 在所属分组内的稳定排序值。 */
  order: number;
  /** 是否只在桌面应用壳中展示。 */
  desktopOnly?: boolean;
}

/** 应用壳一级路由的共享元数据来源。 */
export interface AppRouteCatalogEntry {
  /** 侧栏、移动 Header 和页面主标题共用的名称。 */
  title: string;
  /** 进入页面所需的权限代码；后端仍执行最终授权。 */
  requiredPermission: string;
  /** 进入页面还需同时具备的完整权限组合；未声明时只检查 requiredPermission。 */
  requiredPermissions?: readonly string[];
  /** 路由在应用导航中的呈现规则。 */
  navigation: AppRouteNavigation;
}

/** 应用壳一级路由目录；同一页面名称只能在这里声明一次。 */
export const appRouteCatalog = {
  dashboard: {
    title: "库存总览",
    requiredPermission: stockPermissions.dashboardRead,
    navigation: { group: "primary", icon: "dashboard", order: 10 },
  },
  items: {
    title: "物品管理",
    requiredPermission: stockPermissions.itemRead,
    navigation: { group: "primary", icon: "items", order: 20 },
  },
  inbound: {
    title: "新建入库",
    requiredPermission: stockPermissions.inboundCreate,
    navigation: { group: "primary", icon: "inbound-create", order: 30 },
  },
  "inbound-orders": {
    title: "入库单",
    requiredPermission: stockPermissions.inboundRead,
    navigation: { group: "primary", icon: "inbound-orders", order: 40 },
  },
  outbound: {
    title: "新建出库",
    requiredPermission: stockPermissions.outboundCreate,
    navigation: {
      group: "primary",
      icon: "outbound-create",
      order: 50,
      desktopOnly: true,
    },
  },
  "outbound-orders": {
    title: "出库单",
    requiredPermission: stockPermissions.outboundRead,
    navigation: { group: "primary", icon: "outbound-orders", order: 60 },
  },
  "inbound-approvals": {
    title: "入库审批",
    requiredPermission: stockPermissions.inboundApprove,
    requiredPermissions: [stockPermissions.inboundRead, stockPermissions.inboundApprove],
    navigation: { group: "primary", icon: "inbound-approvals", order: 70 },
  },
  "outbound-approvals": {
    title: "出库审批",
    requiredPermission: stockPermissions.outboundApprove,
    requiredPermissions: [stockPermissions.outboundRead, stockPermissions.outboundApprove],
    navigation: { group: "primary", icon: "outbound-approvals", order: 80 },
  },
  locations: {
    title: "库位管理",
    requiredPermission: stockPermissions.locationRead,
    navigation: { group: "management", icon: "locations", order: 10 },
  },
  templates: {
    title: "分类与模板",
    requiredPermission: stockPermissions.templateRead,
    navigation: { group: "management", icon: "templates", order: 20 },
  },
  substitutes: {
    title: "替代关系",
    requiredPermission: stockPermissions.substituteRead,
    navigation: { group: "management", icon: "substitutes", order: 30 },
  },
  events: {
    title: "审计日志",
    requiredPermission: stockPermissions.auditRead,
    navigation: { group: "management", icon: "events", order: 40 },
  },
  users: {
    title: "用户管理",
    requiredPermission: userPermissions.read,
    navigation: { group: "management", icon: "users", order: 50 },
  },
} as const satisfies Record<string, AppRouteCatalogEntry>;

/** 应用壳一级路由的稳定名称联合。 */
export type AppRouteName = keyof typeof appRouteCatalog;

/** 为 Router 生成应用壳页面元数据，避免标题和权限在路由表中重复声明。 */
export function getAppRouteMeta(routeName: AppRouteName): RouteMeta {
  const entry = appRouteCatalog[routeName];
  return {
    title: entry.title,
    requiresAuth: true,
    requiredPermission: entry.requiredPermission,
    requiredPermissions:
      "requiredPermissions" in entry ? [...entry.requiredPermissions] : [entry.requiredPermission],
    navigation: { ...entry.navigation },
  };
}
