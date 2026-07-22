// 本文件拥有 frontend 路由表和 history 策略；它不实现鉴权状态或平台 WebView 生命周期。
import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";
import AppShell from "../layouts/AppShell.vue";
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
        component: () => import("../pages/DashboardPage.vue"),
        meta: getAppRouteMeta("dashboard"),
      },
      {
        path: "items",
        name: "items",
        component: () => import("../pages/ItemsPage.vue"),
        meta: getAppRouteMeta("items"),
      },
      {
        path: "inbound",
        name: "inbound",
        component: () => import("../pages/InboundDraftPage.vue"),
        meta: getAppRouteMeta("inbound"),
      },
      {
        path: "inbound/orders",
        name: "inbound-orders",
        component: () => import("../pages/InboundOrdersPage.vue"),
        meta: getAppRouteMeta("inbound-orders"),
      },
      {
        path: "outbound",
        name: "outbound",
        component: () => import("../pages/OutboundDraftPage.vue"),
        meta: getAppRouteMeta("outbound"),
      },
      {
        path: "outbound/orders",
        name: "outbound-orders",
        component: () => import("../pages/OutboundOrdersPage.vue"),
        meta: getAppRouteMeta("outbound-orders"),
      },
      {
        path: "approvals/inbound",
        name: "inbound-approvals",
        component: () => import("../pages/InboundApprovalsPage.vue"),
        meta: getAppRouteMeta("inbound-approvals"),
      },
      {
        path: "approvals/outbound",
        name: "outbound-approvals",
        component: () => import("../pages/OutboundApprovalsPage.vue"),
        meta: getAppRouteMeta("outbound-approvals"),
      },
      {
        path: "locations",
        name: "locations",
        component: () => import("../pages/LocationsPage.vue"),
        meta: getAppRouteMeta("locations"),
      },
      {
        path: "templates",
        name: "templates",
        component: () => import("../pages/TemplatesPage.vue"),
        meta: getAppRouteMeta("templates"),
      },
      {
        path: "substitutes",
        name: "substitutes",
        component: () => import("../pages/SubstitutesPage.vue"),
        meta: getAppRouteMeta("substitutes"),
      },
      {
        path: "events",
        name: "events",
        component: () => import("../pages/EventsPage.vue"),
        meta: getAppRouteMeta("events"),
      },
      {
        path: "users",
        name: "users",
        component: () => import("../pages/UsersPage.vue"),
        meta: getAppRouteMeta("users"),
      },
    ],
  },
  {
    path: "/settings/runtime",
    name: "runtime-settings",
    component: () => import("../pages/RuntimeSettingsPage.vue"),
    meta: {
      title: "本机运行设置",
      requiresAuth: false,
      requiresService: false,
      allowsPasswordChangeRequired: true,
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
