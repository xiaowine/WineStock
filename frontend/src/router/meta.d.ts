// 本文件扩展 Vue Router 页面元数据，属于 frontend 路由契约；它不负责执行鉴权判断。
import 'vue-router'

declare module 'vue-router' {
  interface RouteMeta {
    /** 页面标题，供桌面和移动 Shell 展示当前页面上下文。 */
    title: string
    /** 标记页面是否需要登录；真正的守卫将在鉴权基础设施接入后实现。 */
    requiresAuth: boolean
  }
}

export {}
