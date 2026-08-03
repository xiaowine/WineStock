# Desktop Tauri Shell 文档

`desktop` 是 WineStock 正式的 Windows 优先 Tauri v2 桌面壳。它拥有窗口、打包前端资源、
Shell Bridge 传输、运行配置的本地持久化以及 `winestock_core` 本地服务生命周期；不拥有原生设置 UI，
也不代理任何业务 HTTP 请求。

## 运行与构建

在 `frontend/` 执行：

```text
pnpm desktop:dev
pnpm desktop:build
```

Tauri 开发态和发布构建都使用共享前端的普通 Vite 配置，发布时将同一份 `frontend/dist` 打包进应用，
主窗口不会导航到 Axum 或远端 API 地址。

Desktop transport 由 Tauri 运行时识别；`clientKind`、`deviceName` 和 `appVersion` 由 Desktop Rust 在 WebView
document-start 阶段动态注入。设备名由 `whoami` 按当前 Windows、macOS 或 Linux 用户解析，
版本来自 Tauri 包信息，读取失败时设备名回退为 `WineStock Desktop`。

Windows `x86_64-pc-windows-msvc` 目标沿用默认 Rust 链接方式；`core` 和 desktop 壳不会以 DLL 形式交付。
当前 Rustls/AWS-LC 版本链与 KeyWine 对齐，避免额外的 VC++ runtime 文件依赖。Tauri 仍使用 Windows
已安装的 WebView2 Runtime，这是 WebView 宿主组件而不是 Rust 动态库。

Windows 发布只生成 NSIS。NSIS 使用 Tauri 的 `downloadBootstrapper` WebView2 安装模式；目标机器缺少
WebView2 Runtime 时，安装器会联网下载并执行 Microsoft WebView2 bootstrapper。安装器内置 English 和
SimpChinese，默认按 Windows 系统语言自动选择；需要显示语言选择框时，把 `displayLanguageSelector` 改为
`true`。离线安装需要另行切换为 `offlineInstaller` 或固定运行时并重新评估安装包体积。

桌面壳使用 `tauri-plugin-single-instance` 保证只运行一个进程。再次启动时只恢复、取消最小化并聚焦
首个实例的主窗口；第二个实例的参数、工作目录和 URL 均不会转交，插件会关闭第二个实例。默认 WebView
快捷键由 `tauri-plugin-prevent-default` 控制：Debug 使用 `debug()` 保留调试快捷键，Release 使用 `init()`
禁用 WebView2 默认快捷键。

Windows WebView2 的 CDP 远程调试仅由 Debug 构建启用：默认监听 `127.0.0.1:9222`，可用
`WINESTOCK_WEBVIEW2_CDP_PORT` 修改端口。Release 构建会清理 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`，
不接受外部注入调试参数。连接方式、安全边界和 MCP 调试步骤见
[`implementation-notes/webview2-remote-debugging.md`](implementation-notes/webview2-remote-debugging.md)。

Desktop 偏好支持由 `tauri-plugin-autostart` 管理的“开机自启”和“静默启动”。自启动进程带有内部启动标记；
只有该标记与静默偏好同时满足且托盘可用时，主窗口才会在前端首帧后继续隐藏。手动启动、单实例恢复和托盘恢复
仍显示主窗口。实现边界和验收项见 [`implementation-notes/desktop-autostart.md`](implementation-notes/desktop-autostart.md)。

Desktop 偏好还支持关闭后按空闲时间回收主 WebView。回收默认关闭；开启后由前端设置等待时长，Shell 只销毁主
`WebviewWindow`，不停止 `RunningLocalService` 或本地 Axum。托盘和单实例恢复会在窗口不存在时重新创建并加载共享前端；
实现方案和验收项见根文档 [`../../docs/implementation-notes/desktop-webview-idle-reclamation.md`](../../docs/implementation-notes/desktop-webview-idle-reclamation.md)。

Windows 应用启动时由 Rust Shell 调用 WebView2 官方 Loader API 查询实际 Evergreen Runtime 版本（复用 Tauri/Wry 的静态 Loader 绑定），
最低主版本与 Android Shell 对齐为 Chromium M111（配置使用 `111.0.0.0`，不限制补丁号）。WebView2 未安装、无法找到、版本过低、
版本格式异常或检查 API 失败时，不创建主窗口、不加载前端、不启动本地服务；Shell 通过 `rfd` 原生错误对话框提示重新安装 WineStock，
由安装器补全或更新 WebView2。安装器的 `minimumWebview2Version` 同样设置为 `111.0.0.0`。
前端 Shell Bridge 在初始化、原生扩展订阅或首屏握手阶段失败时，不继续显示 WebView；由 `shell_frontend_failed`
接收稳定失败代码，按故障类别显示 `rfd` 原生提示并退出进程。WebView2 和 Shell Bridge 提示正文末尾会追加
受控的 `错误代码：...`，未知前端值降级为通用码；8 秒内未完成握手时显示带
`错误代码：FRONTEND_LOAD_TIMEOUT` 的原生超时提示并退出，不再展示空白窗口。

### Debug 故障注入

先关闭已有 Desktop 实例，然后在 `desktop/` 目录运行以下命令。Tauri CLI 的第二个 `--` 后为应用参数：

```powershell
..\frontend\node_modules\.bin\tauri.cmd dev -- -- --winestock-force-webview-block
..\frontend\node_modules\.bin\tauri.cmd dev -- -- --winestock-force-shell-bridge-block
..\frontend\node_modules\.bin\tauri.cmd dev -- -- --winestock-force-shell-bridge-handshake-block
```

三个参数依次覆盖 WebView2 门禁失败、初始 Shell Bridge 调用失败和首屏就绪握手失败；后两者应隐藏 WebView，
按稳定失败类别使用 `rfd` 显示原生提示并退出。参数只对 Debug 构建生效，Release 构建会忽略。

本地运行配置保存于 Tauri 的 `app_data_dir/config.json`。配置、数据库和文件目录均由本壳管理，绝对路径
不会经 Shell Bridge 返回前端。首次不存在配置时，不写入配置且不启动 core；成功应用本地模式后才持久化实际
端口并由 `DesktopRuntimeManager` 持有 `RunningLocalService` 至停止、替换或退出。

## 实现文档

- [`implementation-notes/desktop-server-mode.md`](implementation-notes/desktop-server-mode.md)：Desktop `server-mode`
  的现状分析、复用边界、实施步骤、Shell Bridge 变化和验收矩阵。
- [`implementation-notes/desktop-firewall-access.md`](implementation-notes/desktop-firewall-access.md)：Desktop
  server-mode 的跨平台 LAN 地址发现、Windows 防火墙规则、UAC 提权、其它系统策略和验收方案。
- [`implementation-notes/webview2-remote-debugging.md`](implementation-notes/webview2-remote-debugging.md)：Windows
  WebView2 Debug/CDP 远程调试策略、MCP 连接方式和 Release 安全边界。
- [`implementation-notes/desktop-background-tray.md`](implementation-notes/desktop-background-tray.md)：Desktop
  关闭窗口转后台、系统托盘恢复主窗口和明确退出的生命周期方案。
- [`implementation-notes/desktop-autostart.md`](implementation-notes/desktop-autostart.md)：Desktop
  开机自启、静默启动、Tauri autostart 插件接入、Shell Bridge 偏好和跨平台验收方案。
- [`implementation-notes/desktop-webview-idle-reclamation.md`](../../docs/implementation-notes/desktop-webview-idle-reclamation.md)：Desktop
  托盘隐藏后的 WebView 空闲回收、偏好设置、窗口重建和 Axum 持续运行方案。
- [`implementation-notes/desktop-startup-gates-warning-remediation.md`](implementation-notes/desktop-startup-gates-warning-remediation.md)：Desktop
  WebView2 版本门卫、Shell Bridge 加载/握手门卫的故障分类、提示文案和验收方案。

## 验证入口

```text
cargo test -p winestock-desktop
cd frontend && pnpm run build
cd frontend && pnpm run test:runtime-funnel
cd frontend && pnpm run test:availability-policy
```

Windows 发布验证还应执行 `pnpm desktop:build`，安装生成的 NSIS 后检查 WebView2 缺失时的在线安装、离线资源加载、
首次设置、local/remote 切换和退出释放端口。
