// 本文件拥有 frontend 路由表和 history 策略；它不实现鉴权状态或平台 WebView 生命周期。
import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";
import AppShell from "../layouts/AppShell.vue";
import { appPageLoaders } from "./appPageLoaders";
import { getAppRouteMeta } from "./appRouteCatalog";

const routes: RouteRecordRaw[] = [
  {
    path: "/",
    component: AppShell,
    meta: {
      title: "WineStock",
      requiresAuth: true,
    },
    children: [
      {
        path: "",
        redirect: { name: "dashboard" },
      },
      {
        path: "dashboard",
        name: "dashboard",
        component: appPageLoaders.dashboard,
        meta: getAppRouteMeta("dashboard"),
      },
      {
        path: "items",
        name: "items",
        component: appPageLoaders.items,
        meta: getAppRouteMeta("items"),
      },
      {
        path: "inbound",
        name: "inbound",
        component: appPageLoaders.inbound,
        props: { kind: "inbound" },
        meta: getAppRouteMeta("inbound"),
      },
      {
        path: "inbound/orders",
        name: "inbound-orders",
        component: appPageLoaders["inbound-orders"],
        meta: getAppRouteMeta("inbound-orders"),
      },
      {
        path: "outbound",
        name: "outbound",
        component: appPageLoaders.outbound,
        props: { kind: "outbound" },
        meta: getAppRouteMeta("outbound"),
      },
      {
        path: "outbound/orders",
        name: "outbound-orders",
        component: appPageLoaders["outbound-orders"],
        meta: getAppRouteMeta("outbound-orders"),
      },
      {
        path: "approvals/inbound",
        name: "inbound-approvals",
        component: appPageLoaders["inbound-approvals"],
        meta: getAppRouteMeta("inbound-approvals"),
      },
      {
        path: "approvals/outbound",
        name: "outbound-approvals",
        component: appPageLoaders["outbound-approvals"],
        meta: getAppRouteMeta("outbound-approvals"),
      },
      {
        path: "locations",
        name: "locations",
        component: appPageLoaders.locations,
        meta: getAppRouteMeta("locations"),
      },
      {
        path: "templates",
        name: "templates",
        component: appPageLoaders.templates,
        meta: getAppRouteMeta("templates"),
      },
      {
        path: "substitutes",
        name: "substitutes",
        component: appPageLoaders.substitutes,
        meta: getAppRouteMeta("substitutes"),
      },
      {
        path: "events",
        name: "events",
        component: appPageLoaders.events,
        meta: getAppRouteMeta("events"),
      },
      {
        path: "users",
        name: "users",
        component: appPageLoaders.users,
        meta: getAppRouteMeta("users"),
      },
    ],
  },
  {
    path: "/setup",
    name: "setup-wizard",
    component: () => import("../pages/SetupWizardPage.vue"),
    meta: {
      title: "欢迎使用 WineStock",
      requiresAuth: false,
      requiresService: false,
    },
  },
  {
    path: "/settings/runtime",
    name: "runtime-settings",
    component: () => import("../pages/RuntimeSettingsPage.vue"),
    meta: {
      title: "运行设置",
      requiresAuth: false,
      requiresService: false,
      allowsPasswordChangeRequired: true,
    },
  },
  {
    path: "/auth",
    name: "auth-entry",
    component: () => import("../pages/AuthEntryPage.vue"),
    meta: {
      title: "准备连接",
      requiresAuth: false,
      requiresService: true,
    },
  },
  {
    path: "/login",
    name: "login",
    component: () => import("../pages/LoginPage.vue"),
    meta: {
      title: "登录",
      requiresAuth: false,
      requiresService: true,
    },
  },
  {
    path: "/register",
    name: "register",
    component: () => import("../pages/RegisterPage.vue"),
    meta: {
      title: "创建首个用户",
      requiresAuth: false,
      requiresService: true,
    },
  },
  {
    path: "/change-password",
    name: "change-password",
    component: () => import("../pages/ChangePasswordPage.vue"),
    meta: {
      title: "修改密码",
      requiresAuth: true,
      requiresService: true,
      allowsPasswordChangeRequired: true,
    },
  },
  {
    // 显式清空 catch-all 参数，避免 pathMatch 被继承到无参数的 dashboard 路由。
    path: "/:pathMatch(.*)*",
    name: "home-fallback",
    redirect: { name: "dashboard", params: {} },
    meta: {
      title: "WineStock",
      requiresAuth: false,
    },
  },
];

/**
 * 前端共享路由器。
 * 使用 hash history，确保平台打包资源不依赖 Axum 或其它服务器提供 SPA fallback。
 */
export const router = createRouter({
  history: createWebHashHistory(import.meta.env.BASE_URL),
  routes,
  scrollBehavior: () => ({ top: 0 }),
});
