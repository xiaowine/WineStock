// 本文件拥有 frontend 路由表和 history 策略；它不实现鉴权状态或平台 WebView 生命周期。
import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'
import { userPermissions } from '../auth/permissions'
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
        },
      },
      {
        path: 'items',
        name: 'items',
        component: () => import('../pages/ItemsPage.vue'),
        meta: {
          title: '物品',
          requiresAuth: true,
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
