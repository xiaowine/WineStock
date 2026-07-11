// 本文件拥有 frontend 路由表和 history 策略；它不实现鉴权状态或平台 WebView 生命周期。
import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'
import { stockPermissions, userPermissions } from '../auth/permissions'
import AppShell from '../layouts/AppShell.vue'

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
        meta: {
          title: '总览',
          requiresAuth: true,
          requiredPermission: stockPermissions.dashboardRead,
        },
      },
      {
        path: 'items',
        name: 'items',
        component: () => import('../pages/ItemsPage.vue'),
        meta: {
          title: '物品',
          requiresAuth: true,
          requiredPermission: stockPermissions.itemRead,
        },
      },
      {
        path: 'inbound',
        name: 'inbound',
        component: () => import('../pages/InboundDraftPage.vue'),
        meta: {
          title: '新建入库',
          requiresAuth: true,
          requiredPermission: stockPermissions.inboundCreate,
        },
      },
      {
        path: 'inbound/orders',
        name: 'inbound-orders',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          title: '入库记录',
          description: '查看入库单列表、筛选结果和单据详情。',
          apiArea: 'GET /api/inbound、/api/inbound/filter-values、/api/inbound/{id}',
        },
        meta: {
          title: '入库记录',
          requiresAuth: true,
          requiredPermission: stockPermissions.inboundRead,
        },
      },
      {
        path: 'outbound',
        name: 'outbound',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          title: '新建出库',
          description: '创建待审批出库单，库存扣减仍由审批流程完成。',
          apiArea: 'POST /api/outbound',
        },
        meta: {
          title: '新建出库',
          requiresAuth: true,
          requiredPermission: stockPermissions.outboundCreate,
        },
      },
      {
        path: 'outbound/orders',
        name: 'outbound-orders',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          title: '出库记录',
          description: '查看出库单列表、筛选结果和单据详情。',
          apiArea: 'GET /api/outbound、/api/outbound/filter-values、/api/outbound/{id}',
        },
        meta: {
          title: '出库记录',
          requiresAuth: true,
          requiredPermission: stockPermissions.outboundRead,
        },
      },
      {
        path: 'approvals/inbound',
        name: 'inbound-approvals',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          title: '入库审批',
          description: '处理待审批入库单的通过与拒绝操作。',
          apiArea: 'POST /api/stock-approvals/inbound/{id}/approve、/reject',
        },
        meta: {
          title: '入库审批',
          requiresAuth: true,
          requiredPermission: stockPermissions.inboundApprove,
        },
      },
      {
        path: 'approvals/outbound',
        name: 'outbound-approvals',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          title: '出库审批',
          description: '处理待审批出库单的通过与拒绝操作。',
          apiArea: 'POST /api/stock-approvals/outbound/{id}/approve、/reject',
        },
        meta: {
          title: '出库审批',
          requiresAuth: true,
          requiredPermission: stockPermissions.outboundApprove,
        },
      },
      {
        path: 'locations',
        name: 'locations',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          title: '库位管理',
          description: '维护库位分组、库位和整批次移库。',
          apiArea: '/api/location-groups、/api/locations、/api/location-transfers',
        },
        meta: {
          title: '库位管理',
          requiresAuth: true,
          requiredPermission: stockPermissions.locationRead,
        },
      },
      {
        path: 'templates',
        name: 'templates',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          title: '分类与模板',
          description: '维护物品分类、物品属性模板和入库模板。',
          apiArea: '/api/item-categories、/api/item-attribute-templates、/api/inbound-templates',
        },
        meta: {
          title: '分类与模板',
          requiresAuth: true,
          requiredPermission: stockPermissions.templateRead,
        },
      },
      {
        path: 'substitutes',
        name: 'substitutes',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          title: '替代料',
          description: '查看并维护物品之间的替代关系与优先级。',
          apiArea: '/api/substitutes、/api/substitutes/{item_id}',
        },
        meta: {
          title: '替代料',
          requiresAuth: true,
          requiredPermission: stockPermissions.substituteRead,
        },
      },
      {
        path: 'events',
        name: 'events',
        component: () => import('../pages/PlaceholderPage.vue'),
        props: {
          title: '事件日志',
          description: '查询用户与库存业务产生的审计事件。',
          apiArea: 'GET /api/events',
        },
        meta: {
          title: '事件日志',
          requiresAuth: true,
          requiredPermission: stockPermissions.auditRead,
        },
      },
      {
        path: 'users',
        name: 'users',
        component: () => import('../pages/UsersPage.vue'),
        meta: {
          title: '用户管理',
          requiresAuth: true,
          requiredPermission: userPermissions.read,
        },
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
      title: '注册',
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
