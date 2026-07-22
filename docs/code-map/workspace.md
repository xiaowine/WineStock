# 工作区代码地图

本文记录仓库根目录、当前实现范围、依赖方向、测试布局和验证入口。

## 当前范围

WineStock 的正式产品目标是多平台，但当前 Rust 实现范围是 server/API 优先。

当前正式 Rust 工作区成员：

- `android/native`
- `core`
- `server`
- `shared`

其它当前目录：

- `frontend` 是 Vue/Vite 共享前端源码区域，不由 Axum 服务。
- `desktop` 是普通 Rust 脚手架，不是工作区成员，也不是正式 Tauri shell。
- `android` 是正式原生 WebView shell；其中 `android/native` 是唯一 JNI Rust 适配 crate。

## 根目录

- `AGENTS.md`：全项目 agent 操作入口，导航到跨组件规范、领域文档、代码地图和完成检查清单。
- `Cargo.toml`：Cargo 工作区成员和共享依赖版本。
- `Cargo.lock`：Rust 依赖锁文件。
- `docs/`：跨组件架构、网络、平台、项目结构、代理清单、跨组件实现笔记和分层代码地图。
- `core/docs/business-api.md` 与 `core/docs/business-api/`：业务 API 文档入口和按业务域拆分的详细接口文档。
- `core/docs/`：数据库、权限、用户管理、core 校验和实现记录。
- `shared/docs/`：共享配置、配置文件加载和基础校验文档。
- `frontend/docs/`：前端页面框架、路由、API client、视觉和后续页面说明。
- `server/docs/`：server shell 文档入口和后续部署、配置说明归属位置。
- `docs/implementation-notes/`：仅保留跨组件的关键方案和历史演进记录。
- `core/`：共享 Rust/Axum 服务库。
- `shared/`：平台无关运行配置、配置解析错误和基础文本校验。
- `server/`：运行共享服务的无头服务端 shell。
- `android/`：Application/Activity、WebView、Shell Bridge、ARM64 JNI 构建和 APK 打包。
- `frontend/`：共享前端源码和 pnpm 工程。
- `desktop/`：非正式普通 Rust 脚手架。

## 工作区依赖方向

允许方向：

```text
server -> core -> shared
server -> shared
android/native -> core -> shared
android app -> packaged frontend assets + android/native
frontend -> HTTP API
```

禁止方向：

```text
shared -> core
core   -> server
core   -> desktop/android/frontend platform assets
core   -> android/native
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
- Android native JSON/config contract、Application 级配置事务、回滚和页面 generation。

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

Android ARM64/APK 验证：

```text
cargo ndk -t arm64-v8a -P 26 check -p winestock-android-native --locked
cd android
gradlew.bat :app:testDebugUnitTest :app:assembleDebug :app:assembleRelease :app:lintDebug --no-daemon
```

当前只验收 APK，且仅允许 `arm64-v8a`；AAB、32 位和 x86 ABI 不属于当前发布面。

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
