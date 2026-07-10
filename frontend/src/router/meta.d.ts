// 本文件扩展 Vue Router 页面元数据，属于 frontend 路由契约；它不负责执行鉴权判断。
import 'vue-router'

declare module 'vue-router' {
  interface RouteMeta {
    /** 页面标题，供桌面和移动 Shell 展示当前页面上下文。 */
    title: string
    /** 标记页面是否需要登录；全局守卫会等待会话初始化后执行判断。 */
    requiresAuth: boolean
    /** 强制改密用户是否允许进入该页面；默认不允许。 */
    allowsPasswordChangeRequired?: boolean
    /** 进入页面所需的权限代码；只用于前端入口控制，后端仍执行实时授权。 */
    requiredPermission?: string
  }
}

export {}
