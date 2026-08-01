# 工作区代码地图

本文记录仓库根目录、当前实现范围、依赖方向、测试布局和验证入口。

## 当前范围与根目录

WineStock 的正式产品目标是多平台，当前 Rust 实现范围是 server/API 优先。
Cargo 工作区成员：`android/native`、`core`、`desktop`、`server`、`shared`。

- `AGENTS.md`：全项目 agent 操作入口，导航到跨组件规范、领域文档、代码地图和完成检查清单。
- `Cargo.toml`/`Cargo.lock`：工作区成员、共享依赖版本、Release profile 和锁文件；Release profile
  对 server 与 Android native 等全部 Rust 产物统一启用 fat LTO，平台打包阶段各自负责最终符号处理；
  当前持久化依赖线为 SeaORM 2.0 / SQLx 0.9，迁移与 feature 决策记录在 `docs/implementation-notes/rust-dependency-remediation.md`。
- `brand/`：WineStock 跨平台矢量母版；frontend、Android 和未来 Shell 只保存各自工具链需要的派生资源，不在平台目录重新设计标志。
- `docs/`：跨组件架构、网络、平台、项目结构、代理清单、分层代码地图和跨组件实现笔记（`docs/implementation-notes/` 只保留跨组件方案与历史演进记录）。
- `core/`、`shared/`、`server/`：共享 Rust/Axum 服务库、平台无关配置 crate 和无头服务端 shell；各自的 `docs/` 拥有组件实现文档（core 业务 API 文档入口为 `core/docs/business-api.md`）。
- `frontend/`：共享前端源码和 pnpm 工程，不由 Axum 服务；`frontend/docs/` 拥有前端规范与页面文档。
- `android/`：正式原生 WebView shell；其中 `android/native` 是唯一 JNI Rust 适配 crate。
- `desktop`：正式 Tauri v2 桌面 shell，负责 Windows 窗口、打包前端、受限 Shell Bridge 与本地 core 生命周期。

## 工作区依赖方向

允许方向：

```text
server -> core -> shared
server -> shared
android/native -> core -> shared
android app -> packaged frontend assets + android/native
desktop -> core -> shared
desktop -> packaged frontend assets
frontend -> HTTP API
frontend/android/future shells -> brand vector masters (build-time derivation only)
```

禁止方向：

```text
shared -> core
core   -> server
core   -> desktop/android/frontend platform assets
core   -> android/native
brand  -> frontend/android platform code
```

`brand` 只提供静态设计母版，不依赖任何运行时组件；上面的禁止方向表示母版不得反向依赖平台实现。

## 测试布局

Rust 单元测试统一放在各 crate 的 `src/tests/` 目录中，源码文件只保留测试模块声明。
测试仍作为被测模块的子模块挂载，因此可以访问本模块私有项；`core/src/tests/support.rs` 复用测试搭建逻辑。

当前主要测试覆盖：core 启动/HTTP/鉴权/用户与全部库存子域、persistence 连接与 repository（包括 SQLite RETURNING 所需的 `>= 3.35` 运行时门槛）、server shell 生命周期与配置、shared 配置解析与基础规则、Android native contract 与配置事务，以及 desktop Tauri runtime manager 的配置、端口与 core 生命周期。

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

Android ARM64/APK 验证（当前只验收 APK，仅允许 `arm64-v8a`）：

```text
cargo ndk -t arm64-v8a -P 28 check -p winestock-android --locked
cd android
gradlew.bat :app:testDebugUnitTest :app:assembleDebug :app:assembleRelease :app:lintDebug --no-daemon
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
