# WineStock Frontend

本目录保存 WineStock 的 Vue 共享前端源码。
Desktop Tauri 与 Android 平台 shell 可以复用源码，但分别负责资源打包和 WebView 生命周期；Axum 不服务前端构建产物。

第一次接手前端时，先阅读 [`docs/README.md`](docs/README.md) 了解当前页面、路由、API client 和视觉文档入口。

## 构建环境变量

Microsoft Clarity 使用 `VITE_CLARITY_PROJECT_ID`。在 `frontend/.env` 中填写该变量即可，Vite 会在构建时自动加载；`.env` 未提供时 Clarity 保持停用。

Clarity 项目 ID 必须视为浏览器公开配置：Vite 会把它编译进客户端产物，因此本地 `.env` 用于避免把值提交到开源源码，不提供运行时秘密保护。`.env.example` 只提供空值模板。

## 开发命令

```powershell
pnpm install
pnpm dev
pnpm build
```

## 当前结构

- `src/api/`：运行时服务地址、通用 HTTP 请求、错误类型和业务 API 契约。
- `src/auth/`：内存 access token、统一 localStorage refresh token、会话恢复和 token 轮换。
- `src/router/`：路由表、路由元数据与一级导航。
- `src/layouts/`：桌面和移动响应式应用壳。
- `src/pages/`：页面路由入口。
- `src/styles/`：浅色主题和布局样式。

当前使用 hash history，具体原因与路由清单见 [`docs/routes.md`](docs/routes.md)。
