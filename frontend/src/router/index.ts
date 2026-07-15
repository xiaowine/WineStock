// 本文件拥有 frontend 路由表和 history 策略；它不实现鉴权状态或平台 WebView 生命周期。
import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'
import AppShell from '../layouts/AppShell.vue'
import { getAppRouteMeta } from './appRouteCatalog'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: AppShell,
    meta: {
      title: 'WineStock',
      requiresAuth: true,
    },
    children: [
      {
        path: '',
        redirect: { name: 'dashboard' },
      },
      {
        path: 'dashboard',
        name: 'dashboard',
        component: () => import('../pages/DashboardPage.vue'),
        meta: getAppRouteMeta('dashboard'),
      },
      {
        path: 'items',
        name: 'items',
        component: () => import('../pages/ItemsPage.vue'),
        meta: getAppRouteMeta('items'),
      },
      {
        path: 'inbound',
        name: 'inbound',
        component: () => import('../pages/InboundDraftPage.vue'),
        meta: getAppRouteMeta('inbound'),
      },
      {
        path: 'inbound/orders',
        name: 'inbound-orders',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          description: '查看入库单列表、筛选结果和单据详情。',
          apiArea: 'GET /api/inbound、/api/inbound/filter-values、/api/inbound/{id}',
        },
        meta: getAppRouteMeta('inbound-orders'),
      },
      {
        path: 'outbound',
        name: 'outbound',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          description: '创建待审批出库单，库存扣减仍由审批流程完成。',
          apiArea: 'POST /api/outbound',
        },
        meta: getAppRouteMeta('outbound'),
      },
      {
        path: 'outbound/orders',
        name: 'outbound-orders',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          description: '查看出库单列表、筛选结果和单据详情。',
          apiArea: 'GET /api/outbound、/api/outbound/filter-values、/api/outbound/{id}',
        },
        meta: getAppRouteMeta('outbound-orders'),
      },
      {
        path: 'approvals/inbound',
        name: 'inbound-approvals',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          description: '处理待审批入库单的通过与拒绝操作。',
          apiArea: 'POST /api/stock-approvals/inbound/{id}/approve、/reject',
        },
        meta: getAppRouteMeta('inbound-approvals'),
      },
      {
        path: 'approvals/outbound',
        name: 'outbound-approvals',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          description: '处理待审批出库单的通过与拒绝操作。',
          apiArea: 'POST /api/stock-approvals/outbound/{id}/approve、/reject',
        },
        meta: getAppRouteMeta('outbound-approvals'),
      },
      {
        path: 'locations',
        name: 'locations',
        component: () => import('../pages/LocationsPage.vue'),
        meta: getAppRouteMeta('locations'),
      },
      {
        path: 'templates',
        name: 'templates',
        component: () => import('../pages/TemplatesPage.vue'),
        meta: getAppRouteMeta('templates'),
      },
      {
        path: 'substitutes',
        name: 'substitutes',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          description: '查看并维护物品之间的替代关系与优先级。',
          apiArea: '/api/substitutes、/api/substitutes/{item_id}',
        },
        meta: getAppRouteMeta('substitutes'),
      },
      {
        path: 'events',
        name: 'events',
        component: () => import('../pages/EventsPage.vue'),
        meta: getAppRouteMeta('events'),
      },
      {
        path: 'users',
        name: 'users',
        component: () => import('../pages/UsersPage.vue'),
        meta: getAppRouteMeta('users'),
      },
    ],
  },
  {
    path: '/login',
    name: 'login',
    component: () => import('../pages/LoginPage.vue'),
    meta: {
      title: '登录',
      requiresAuth: false,
    },
  },
  {
    path: '/register',
    name: 'register',
    component: () => import('../pages/RegisterPage.vue'),
    meta: {
      title: '创建首个用户',
      requiresAuth: false,
    },
  },
  {
    path: '/change-password',
    name: 'change-password',
    component: () => import('../pages/ChangePasswordPage.vue'),
    meta: {
      title: '修改密码',
      requiresAuth: true,
      allowsPasswordChangeRequired: true,
    },
  },
  {
    // 显式清空 catch-all 参数，避免 pathMatch 被继承到无参数的 dashboard 路由。
    path: '/:pathMatch(.*)*',
    name: 'home-fallback',
    redirect: { name: 'dashboard', params: {} },
    meta: {
      title: 'WineStock',
      requiresAuth: false,
    },
  },
]

/**
 * 前端共享路由器。
 * 使用 hash history，确保平台打包资源不依赖 Axum 或其它服务器提供 SPA fallback。
 */
export const router = createRouter({
  history: createWebHashHistory(import.meta.env.BASE_URL),
  routes,
  scrollBehavior: () => ({ top: 0 }),
})
