# 工作区代码地图

本文记录仓库根目录、当前实现范围、依赖方向、测试布局和验证入口。

## 当前范围

WineStock 的正式产品目标是多平台，但当前 Rust 实现范围是 server/API 优先。

当前正式 Rust 工作区成员：

- `core`
- `server`
- `shared`

其它当前目录：

- `frontend` 是 Vue/Vite 共享前端源码区域，不由 Axum 服务。
- `desktop` 是普通 Rust 脚手架，不是工作区成员，也不是正式 Tauri shell。
- 正式 Android shell 代码目前不存在。

## 根目录

- `AGENTS.md`：agent 操作指南。
- `Cargo.toml`：Cargo 工作区成员和共享依赖版本。
- `Cargo.lock`：Rust 依赖锁文件。
- `docs/`：架构、网络、平台、项目结构、业务 API、验证约束、实现笔记和分层代码地图。
- `docs/business-api.md` 与 `docs/business-api/`：业务 API 文档入口和按业务域拆分的详细接口文档。
- `docs/frontend/`：前端页面框架、路由、API client、视觉和后续页面说明。
- `docs/validation/`：按源码实体归档的字段限制、校验入口和数据库约束。
- `docs/implementation-notes/`：关键实现方案和历史演进记录。
- `core/`：共享 Rust/Axum 服务库。
- `shared/`：平台无关运行配置、配置解析错误和基础文本校验。
- `server/`：运行共享服务的无头服务端 shell。
- `frontend/`：共享前端源码和 pnpm 工程。
- `desktop/`：非正式普通 Rust 脚手架。

## 工作区依赖方向

允许方向：

```text
server -> core -> shared
server -> shared
frontend -> HTTP API
```

禁止方向：

```text
shared -> core
core   -> server
core   -> desktop/android/frontend platform assets
```

## 测试布局

Rust 单元测试统一放在各 crate 的 `src/tests/` 目录中，源码文件只保留测试模块声明。
测试仍作为被测模块的子模块挂载，因此可以访问本模块私有项；`core/src/tests/support.rs` 复用测试搭建逻辑。

当前主要测试覆盖：

- core 启动、HTTP/OpenAPI、授权、登录、refresh、logout、注册和用户管理。
- 库存物品、模板、库位、入库、出库、看板、替代料和事件日志。
- persistence 连接、repository 和服务绑定。
- server shell 生命周期与配置。
- shared 配置解析和基础规则。

## 验证入口

默认使用覆盖被改代码路径的最窄命令。

服务端 shell 小改优先使用：

```text
cargo +stable check -p winestock-server
```

跨 crate、公共 API、依赖/features、发布准备或用户明确要求时使用：

```text
cargo +stable check --workspace --all-targets
cargo +stable test --workspace
cargo +stable build -p winestock-server
```

Rust 格式检查：

```text
cargo +stable fmt --all -- --check
```

前端类型与生产构建：

```text
cd frontend
pnpm build
```

本地 server smoke test：

```text
cargo +stable run -p winestock-server
```
