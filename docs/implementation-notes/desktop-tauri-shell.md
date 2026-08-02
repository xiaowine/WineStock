# Desktop Tauri Shell 基础实施方案

> 文档状态：已实施（Windows 安装包 smoke 待执行）  
> 涉及组件：`desktop`、`frontend`、`core`、`shared`
> 编制日期：2026-07-31  
> 实施范围：Windows 优先的 Tauri v2 基础桌面壳

## 1. 目标

建立正式的 `desktop`，复用现有 Vue/Vite 前端、Shell Bridge v1 和
`winestock_core::start_local_service()`，完成以下基础能力：

- 打包并加载 `frontend/dist`，不让 Axum 托管前端资源。
- 首次无配置时打开现有设置漏斗，不写配置、不启动本地服务。
- 支持 `self-hosted`、`client-only` 和 `connect-to-remote`。
- 通过 Tauri command 和 event 实现现有 Shell Bridge v1。
- 在 Tauri 进程内启动、停止和重启共享 Axum core。
- 退出应用前优雅停止本地服务并释放端口。
- 通过单实例插件保证只有一个桌面进程；后续启动只聚焦首个主窗口并退出自身，不转交参数或 URL。
- Debug 保留 WebView2 默认快捷键，Release 禁用默认快捷键。
- 生成可安装的 Windows 桌面构建。

第一版以可用、可测试为目标，不追求完整桌面系统集成。

## 2. 非目标

第一版不实现：

- `winestock-server` sidecar 或第二套服务实现。
- 原生设置窗口、原生错误对话框或业务 API 代理。
- Android `server-mode`、防火墙配置和公网部署；Desktop LAN `server-mode` 由独立实现文档补充。
- 系统托盘、开机启动、自动更新、多窗口和文件关联。
- macOS、Linux 安装包验收。

Desktop capability 中 `serverMode=true`、`nativeBack=false`；Android 和 Web fallback 仍按各自壳能力关闭
`serverMode`。Desktop LAN server-mode 的地址发现和验收见独立实现文档。

## 3. 所有权与数据流

```text
Tauri WebView
  -> 加载 Tauri 打包的 frontend/dist
  -> 通过 @tauri-apps/api 调用 Shell Bridge command
  -> 通过 HTTP 调用本地或远端 Axum 业务 API

desktop
  -> 持久化运行配置
  -> 管理 RunningLocalService
  -> 发布 RuntimeSnapshot 事件

winestock-core -> Axum 服务、业务和存储
winestock-shared -> AppConfig、运行模式和权威配置校验
```

Desktop Shell 直接依赖 `core` 和 `shared`。`core` 不得依赖 Tauri，也不得接收前端构建产物。

## 4. 目录与依赖

正式目录使用：

```text
desktop/
  Cargo.toml
  build.rs
  tauri.conf.json
  capabilities/
    main.json
  permissions/
    shell-bridge.toml
  icons/
  src/
    main.rs
    commands.rs
    contract.rs
    runtime.rs
```

- Cargo package 使用 `winestock-desktop`，并加入根 Cargo workspace。
- Rust 依赖至少包括 Tauri v2、Serde、Tokio、`winestock-core` 和 `winestock-shared`。
- 前端增加匹配 Tauri v2 的 `@tauri-apps/api` 和 `@tauri-apps/cli`。
- Tauri CLI 负责在开发前启动 Vite、在打包前构建前端。
- `frontendDist` 指向仓库 `frontend/dist`（相对 Tauri 配置为 `../../frontend/dist`）；`devUrl` 只指向 Vite 开发服务器，不是 Axum API 地址。

不沿用当前 `desktop/` Hello World 的包名或入口结构；实现时将其替换为上述正式 crate，并清理失效占位内容。

## 5. Desktop Runtime Manager

`runtime.rs` 提供一个进程级 `DesktopRuntimeManager`，由 `tauri::Builder::manage` 注册为 managed state。
管理器串行处理配置和服务变更，至少持有：

- 当前配置状态和 `RuntimeSnapshot`。
- 当前有效 `AppConfig`。
- `Option<RunningLocalService>`。
- 配置文件与应用数据目录。
- 正在退出标记，避免重复执行关闭流程。

配置文件放在 Tauri `app_data_dir` 下。数据库和文件仓也放在该目录，并由 Shell 将平台路径写入
`AppConfig.storage`；绝对路径不通过 Shell Bridge 暴露给前端。

启动规则：

1. 配置文件不存在时返回 `initialized=false`、`unconfigured`、`stopped`，默认配置只作为表单草稿。
2. 配置有效且为本地模式时，在创建并显示主窗口前同步恢复 core；启动过程中的 `starting` 状态不作为首个前端快照，最终快照再交给前端。
3. 配置有效且为远端模式时不启动 core，直接发布 `remote` 快照。
4. 配置损坏时仍加载打包前端，返回 `invalid` 和稳定错误，不覆盖原文件。

## 6. 配置应用

沿用现有 Android 与 Shell Bridge 的事务语义：

```text
validate
  -> 保留旧配置和旧服务状态
  -> 停止需要替换的旧服务
  -> 激活候选配置
  -> 成功后持久化并发布快照
  -> 失败时尽力恢复旧配置和旧服务
```

- 配置校验必须调用 `shared` 的权威规则，前端校验只用于即时提示。
- 首次 `self-hosted` 使用端口 `0` 自动分配；成功后持久化实际端口。
- 已保存的本地端口被占用时，只自动用端口 `0` 重试一次。
- `running` 快照中的端口、`boundAddress` 和 `apiBaseUrl` 必须一致，API 地址固定使用
  `http://127.0.0.1:<实际端口>`。
- 远端 URL 格式有效即可保存，不因远端暂时离线而拒绝配置。
- 配置写入使用同目录临时文件替换，避免中途失败留下半个 JSON 文件。
- `localAuthExchangeToken` 只放入当前本地运行快照，不记录日志、不写配置文件。

## 7. Shell Bridge

Rust 侧只注册现有契约需要的具名 command：

```text
shell_get_runtime_snapshot
shell_validate_runtime_config
shell_apply_runtime_config
shell_start_local_service
shell_stop_local_service
shell_restart_local_service
shell_frontend_ready
```

状态变化通过单向事件发布：

```text
winestock-runtime-state-changed
winestock-app-resumed
```

前端新增 `frontend/src/shell/transports/tauri.ts`，使用 `invoke` 和 `listen` 实现现有 `ShellBridge` 接口，
并直接使用 `@tauri-apps/plugin-opener` 的 `openUrl` 打开外部链接；
`frontend/src/shell/transportFactory.ts` 负责选择注入桥、Tauri 传输或 Web fallback；Tauri 传输通过官方运行时
宿主探测选择，普通浏览器继续使用 Web fallback，不通过 User-Agent 猜测能力。

`openExternal` 只接受业务声明的 `http` 和 `https` URL；Desktop 由 Tauri opener capability 的精确 scope
限制为 GitHub 项目页、QQ群链接和 Microsoft 隐私声明，普通浏览器仍由 Web fallback 处理。
所有业务请求仍由前端 HTTP client 直接访问 Axum。

## 8. Tauri 安全配置

- `permissions/shell-bridge.toml` 只定义上述自有 command 的 allow permission。
- `capabilities/main.json` 只绑定主窗口，并授予 Shell Bridge、事件监听和 opener 精确 URL scope。
- 不启用通用 shell 执行、任意文件系统访问或宽泛 URL opener 权限。
- CSP 至少允许 Tauri 打包资源、loopback HTTP API 和配置的远端 HTTP/HTTPS API。
- 远端 API 范围如无法在静态 CSP 中安全限定，应先使用项目明确允许的连接规则，不得直接关闭 CSP。
- 主窗口只加载 Tauri 打包资源或开发期 Vite 地址，不导航到业务远端页面。

## 9. 生命周期

- Tauri setup 阶段先注册 state、command 和 event，再创建或显示主窗口。
- 首次未配置、远端模式或启动失败时前端仍可直接加载；已有有效本地配置会在主窗口显示前完成一次 Axum 恢复，
  避免把 `configured + stopped` 中间快照误判为运行设置，服务状态仍由快照驱动设置和恢复页面。
- 主窗口创建时保持隐藏，收到前端 `frontendReady`（首帧已渲染）后才显示，避免 WebView 加载过程闪烁；
  8 秒未收到信号时由 Shell 受控显示窗口，避免前端或桥异常造成永久隐藏。
- 窗口重新获得焦点时发布 `winestock-app-resumed`，由前端执行既有恢复检查。
- 第一次收到退出请求时阻止立即退出，异步停止 `RunningLocalService`；完成后再结束进程。
- 使用退出标记避免第二次退出请求重复停止服务。
- core 异常结束时转为 `service_crashed` 快照并发布状态事件，不关闭前端窗口。

## 10. 实施顺序

1. 创建 `desktop` crate、Tauri 配置、图标和最小窗口，接入 Cargo workspace。
2. 接入 Vite 开发与生产构建，确认离线加载打包前端。
3. 实现 Desktop runtime manager、配置读取和首次未初始化快照。
4. 实现 Shell Bridge command、事件和前端 Tauri 适配层。
5. 接入本地 core 启停、动态端口、配置持久化和失败恢复。
6. 实现退出时 graceful shutdown、capabilities 和受限外链。
7. 完成自动化测试、Windows 开发态 smoke 和安装包 smoke。

## 11. 最小验收

必须通过以下场景：

- 无配置首次启动：前端设置漏斗可见，不创建配置、不启动 Axum。
- 应用本地模式：启动 core，返回真实 loopback 地址，健康检查和本机静默登录正常。
- 二次冷启动：读取已保存配置并自动恢复本地服务。
- 应用远端模式：保存配置但不启动本地 core；远端离线时设置页仍可用。
- 本地端口冲突：自动换端口一次，快照和持久配置使用同一实际端口。
- 配置损坏或数据库启动失败：前端仍可进入运行设置并显示稳定错误。
- 本地与远端切换：旧 API 会话按现有前端规则清理，不向新地址发送旧 token。
- 关闭窗口：等待 Axum 停止，原端口可以再次绑定。
- Release 安装包离线启动：前端资源、图标和基础窗口正常。
- Tauri capabilities 不包含通用 shell 和无边界文件系统权限。

单实例和快捷键策略：

- `tauri-plugin-single-instance` 的回调只调用主窗口的显示、取消最小化和聚焦操作；`args`、`cwd` 不被读取，
  因而不会把第二个实例的参数、深链或 URL 注入首个实例。
- `tauri-plugin-prevent-default::debug()` 只用于 Debug 构建；Release 构建使用
  `tauri-plugin-prevent-default::init()`，禁用 WebView2 默认快捷键。

建议执行的最窄检查：

```text
cargo test -p winestock-desktop
cd frontend && pnpm run build
cd frontend && pnpm run test:runtime-funnel
cd frontend && pnpm run test:availability-policy
Tauri development smoke
Tauri Windows release build and installed-app smoke
```

实现完成时同步更新 `docs/platforms.md`、`docs/project-structure.md`、workspace/desktop/frontend
代码地图和对应组件文档，并记录实际执行结果与未覆盖平台。

## 12. WebView 内核版本门禁

三类 UI 平台的内核版本号不能直接按同一套 Chromium 主版本解释：

- Windows Tauri 使用 WebView2，版本为四段 Chromium/Edge 版本，当前最低主版本为 M111，配置为 `111.0.0.0`；
- Android 使用系统 WebView，同样基于 Chromium，可按 M111 主版本判定；
- macOS Tauri 使用系统 WKWebView/WebKit，版本随 macOS/WebKit 发布，不存在 Chromium M111；
- Linux Tauri 使用 WebKitGTK，版本为 WebKitGTK 库版本（例如 `2.x`），也不能直接换算为 Chromium M 版本。

Windows 门禁在创建主窗口前调用 WebView2 官方 Loader API；低版本不创建 WebView、不加载前端、不启动 core，
通过 `rfd` 显示“依赖损坏，请重新安装软件”的跨平台原生错误对话框后退出。macOS/Linux 后续应分别接入 WKWebView/WebKitGTK 原生版本 API，
再复用相同的 `supported` 判定边界。

## 12. 参考依据

- 项目规范：`docs/architecture.md`、`docs/platforms.md`、`docs/runtime-networking.md`、
  `docs/shell-bridge.md`。
- Tauri v2 Vite 集成：<https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/start/frontend/vite.mdx>。
- Tauri command：<https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/develop/calling-rust.mdx>。
- Tauri managed state：<https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/develop/state-management.mdx>。
- Tauri IPC event：<https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/concept/Inter-Process%20Communication/index.mdx>。
- Tauri permissions：<https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/security/permissions.mdx>。
- Tauri capabilities：<https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/security/capabilities.mdx>。

## 13. 实施记录（2026-08-01）

- 已创建 `winestock-desktop` workspace crate、Tauri 配置、Windows 图标、主窗口 capability 与自有
  Shell Bridge permission；主窗口仅加载 Vite 开发地址或 Tauri 打包的 `frontend/dist`。
- `DesktopRuntimeManager` 负责 app-data 下的可编辑配置原子写入、平台派生的数据库/文件路径、首次
  未初始化快照、local/remote 应用事务、端口冲突一次动态重试、异常退出快照及退出前 graceful shutdown。
  `RunningLocalService` 由 manager 持有，服务不会因启动函数返回而提前停止。
- 前端已增加 Desktop Vite mode 与 `src/shell/transports/tauri.ts`，通过 `invoke`、`listen` 对接现有 Shell Bridge v1；
  浏览器仍只使用明确的 Web fallback，不以 User-Agent 或 Tauri 全局对象猜测平台。
- 已补充无 WebView 的 runtime manager 集成测试，覆盖首次未配置、local/remote、端口冲突、损坏配置和
  冷启动自动恢复。Windows 集成测试目标复用 Tauri 生成的 `resource.lib` manifest，并在 Tokio 多线程
  runtime 中运行；当前 8 个测试已全部通过。Tauri Release EXE 已成功生成，NSIS 安装程序
  实际安装/启动 smoke 仍待完成。
- Windows 依赖链已按 KeyWine 对齐到 `rustls 0.23.40`、`aws-lc-rs 1.17.0` 和 `aws-lc-sys 0.41.0`，
  未使用全局静态 CRT 配置；检查 Windows EXE 导入表时应确认不出现额外的 `VCRUNTIME140.dll` 或项目 DLL。
  集成测试曾因未链接 Tauri manifest 返回 `STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)`；现已由构建脚本为
  Windows 测试目标补充同一份 `resource.lib`，并改用 Tokio runtime，问题已排除。
- Windows 发布目标已切换为 NSIS，配置使用 Tauri 的 `downloadBootstrapper` WebView2 安装模式；缺少
  WebView2 Runtime 时由 NSIS 联网下载并执行 Microsoft bootstrapper。安装器内置 English 与 SimpChinese，
  默认按 Windows 系统语言自动选择，必要时可启用 `displayLanguageSelector` 显示选择框。
- 已接入 `tauri-plugin-single-instance` 和 `tauri-plugin-prevent-default`：后续实例只聚焦首个窗口并退出，
  不转交参数或 URL；Debug 保留默认快捷键，Release 禁用 WebView2 默认快捷键。
