# Desktop 关闭窗口转后台实现方案

## 结论

Desktop 主窗口点击关闭后不退出进程，而是隐藏窗口并继续在系统托盘运行；用户通过托盘图标或托盘菜单重新打开主窗口。只有用户选择托盘菜单中的“退出”时，应用才执行现有的本地服务优雅停止流程并结束进程。

该行为属于 Tauri desktop shell 的窗口生命周期，不改变前端路由、Shell Bridge 契约、运行配置或 `winestock_core` 的生命周期语义。窗口隐藏期间，server-mode 仍继续监听端口，其他设备可以继续访问服务。

## 实施状态

本方案已经落地：desktop shell 已提供托盘装配、主窗口隐藏/恢复、关闭行为偏好持久化和前端偏好设置入口。当前已通过 Rust/前端构建与纯逻辑测试；Windows 托盘、窗口关闭、server-mode 持续访问和托盘退出仍需在目标机器上执行实机 smoke。

## 实施前现状基线

当前入口位于 [`desktop/src/main.rs`](../../src/main.rs)：

- 主窗口由 `WebviewWindowBuilder` 创建，初始 `visible(false)`，正常路径仅由 `frontendReady` 显示；8 秒未完成握手时由原生壳提示超时并退出；
- `on_window_event` 目前只处理主窗口获得焦点，用于发布应用恢复事件；没有处理 `WindowEvent::CloseRequested`；
- `RunEvent::ExitRequested` 会阻止立即退出，等待 `DesktopRuntimeManager::shutdown_local_service` 完成后再调用 `handle.exit(0)`；
- `tauri-plugin-single-instance` 的二次启动回调会显示、取消最小化并聚焦主窗口；
- 实施前没有 `TrayIconBuilder`、托盘菜单或托盘图标初始化；
- Tauri 配置已有应用图标资源，可作为 Windows/Linux 托盘图标。macOS 后续应提供适合菜单栏显示的模板图标，不应直接假设彩色应用图标在深浅菜单栏中都可读。

因此，当前直接在关闭事件中调用 `window.hide()` 还不完整：如果没有托盘恢复入口，窗口隐藏后用户无法通过正常 UI 找回；如果把隐藏误实现为停止服务，也会破坏 server-mode 的持续提供服务语义。

## KeyWine 参考与本项目取舍

参考项目 `D:\Project\kb\KeyWine` 的核心原理是：

1. 初始化托盘图标和菜单；
2. 在 `CloseRequested` 中调用 `api.prevent_close()`，再隐藏窗口；
3. 托盘左键恢复、取消最小化并聚焦主窗口；
4. 用独立的退出标志区分“用户关闭窗口”和“用户明确退出”；明确退出时允许窗口关闭或直接进入退出流程；
5. 单实例回调同样恢复首个实例的主窗口。

WineStock 不照搬其应用设置、通知、标题栏和业务状态，而只采用窗口/托盘生命周期原理，并接入现有的 `APP_HANDLE`、`EXIT_REQUESTED`、首屏隐藏和 `DesktopRuntimeManager`。开机自启和自启动静默属于独立的后续方案，见 [`desktop-autostart.md`](desktop-autostart.md)，不在本托盘方案的实施范围内。

## 目标行为

### 关闭行为偏好

在前端偏好设置中增加“关闭窗口时”选项，提供两个互斥值：

```text
minimize-to-tray  最小化到系统托盘
exit-application  退出应用
```

默认使用“最小化到系统托盘”，保持本方案的默认后台运行行为。用户选择“退出应用”后，点击窗口关闭按钮会进入现有的 `ExitRequested` 清理流程；这不是普通的最小化，也不会保留本地 Axum 服务。

这个偏好只控制主窗口的系统关闭请求，不影响：

- 托盘菜单中的“打开 WineStock”；
- 托盘菜单中的“退出 WineStock”，该菜单始终表示明确退出；
- 应用崩溃、系统关机或操作系统强制终止；
- 前端路由、运行模式、登录状态和运行配置。

server-mode 下选择“退出应用”时，关闭窗口会停止本机服务，其他设备将立即无法继续访问。偏好设置中应使用清晰的选项文案；如果用户正在 server-mode，切换为“退出应用”时应通过现有 Notice 或确认交互明确告知这一影响，不新增会挤压页面容器的常驻错误文本。

### 偏好设置的所有权

该值是 desktop 窗口行为，不属于 `EditableRuntimeConfig` 或共享 `AppConfig`，也不应同步到 Android、Server Shell 或远端服务。建议由 desktop shell 持久化到 app data 目录下独立的 desktop preferences 文件，并在 Tauri 状态中缓存当前有效值：

```json
{
  "version": 1,
  "close_behavior": "minimize-to-tray"
}
```

前端通过具名 Shell Bridge 方法读取和更新它，例如 `get_desktop_preferences` 与 `set_desktop_preferences`。这两个 command 只传递有限枚举，不接受任意 JSON、路径或命令参数；对应 capability 也只授予主窗口。设置成功后先更新 shell 内存状态，再持久化文件，写入失败必须返回稳定错误，不能让 UI 显示未保存的值。

关闭事件不能每次从磁盘读取偏好，否则会把 I/O 和解析失败引入窗口事件路径。`CloseRequested` 只读取由 shell 管理的线程安全状态；应用启动时加载失败则使用默认的“最小化到系统托盘”，并记录可诊断日志。

### 普通关闭

用户点击系统窗口关闭按钮时：

```text
CloseRequested
  -> 判断关闭行为偏好
  -> minimize-to-tray：判断是否正在执行明确退出
  -> minimize-to-tray 且未退出：prevent_close
  -> minimize-to-tray：隐藏主窗口
  -> minimize-to-tray：进程、托盘、本地服务继续运行
  -> exit-application：进入现有 ExitRequested 优雅退出流程
```

选择“最小化到系统托盘”时，关闭操作不应：

- 停止或重启 `DesktopRuntimeManager`；
- 删除防火墙规则；
- 清除登录状态或前端页面状态；
- 触发第二次 UAC；
- 写入新的运行配置；
- 创建第二个进程。

选择“退出应用”时，上述“不应”约束不适用；关闭请求应按明确退出处理，但仍必须等待本地服务优雅停止，不能直接终止进程。

### 托盘恢复

托盘图标左键释放，以及托盘菜单中的“打开 WineStock”，统一调用现有的 `show_main_window`：

- `show()`；
- `unminimize()`；
- 确保窗口恢复到任务栏（若实现使用了 `set_skip_taskbar(true)` 隐藏任务栏项，则这里必须恢复为 `false`）；
- `set_focus()`；
- 发布现有的应用恢复事件，由前端按现有机制刷新运行状态。

恢复逻辑应保持幂等。窗口已经可见、最小化、隐藏或焦点在其它应用时，重复点击托盘都应得到同一结果。

### 明确退出

托盘菜单保留“退出 WineStock”。明确退出必须复用当前退出清理路径：

```text
托盘“退出”
  -> 标记允许关闭/正在退出
  -> 触发现有 ExitRequested 收敛流程
  -> prevent_exit
  -> shutdown_local_service
  -> handle.exit(0)
```

实现时不能简单在托盘回调中调用 `window.close()` 后绕过当前 `ExitRequested` 处理，也不能提前把 `EXIT_REQUESTED` 设置为已完成状态，否则会导致本地 Axum 未优雅停止。建议把“允许窗口关闭”和“退出流程已开始”拆成两个清晰状态，或提供一个统一的 `request_application_exit` 辅助函数，确保托盘退出、系统退出和已有退出路径不会重复停止服务。

## 推荐实现结构

### 1. 增加 desktop 托盘模块

建议新增 `desktop/src/tray.rs`，职责限定为：

- 使用 Tauri v2 `TrayIconBuilder` 创建托盘图标；
- 构造“打开”和“退出”两个固定菜单项；
- 处理托盘图标点击和菜单事件；
- 调用主入口提供的窗口恢复/退出辅助函数。

托盘模块不读取运行配置、不调用业务 API、不操作防火墙，也不直接管理 `DesktopRuntimeManager`。这样托盘交互与服务生命周期仍保持边界清晰。

Tauri v2 将旧版统一托盘事件拆分为 `on_menu_event` 和 `on_tray_icon_event`。当前锁定的 Tauri 版本为 `2.11.5`，推荐使用官方高层 API：

```rust
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
```

托盘图标优先取 `app.default_window_icon()`，避免重复维护平台图标。macOS 若需要菜单栏模板图标，再在 desktop 图标目录增加专用资源，并通过 `cfg(target_os = "macos")` 选择。

### 2. 收敛窗口恢复函数

将当前 `show_main_window` 作为所有恢复入口的唯一实现，至少由以下路径复用：

- `tauri-plugin-single-instance` 二次启动回调；
- 托盘左键点击；
- 托盘“打开”菜单；
- 后续可能增加的受控原生恢复入口。

该函数不应负责启动服务或读取配置。它只操作主窗口，并在恢复后调用已有的应用恢复事件发布逻辑；窗口焦点事件仍可作为兜底，不应重复执行高成本初始化。

### 3. 在现有窗口事件中拦截关闭

扩展 `main.rs` 的 `on_window_event`：

- 只处理 `main` 窗口；
- 收到 `CloseRequested` 时先读取 desktop preferences 的有效 `close_behavior`；
- `minimize-to-tray` 下，若没有明确退出许可，则 `api.prevent_close()` 并隐藏窗口；
- `exit-application` 下，不拦截窗口关闭，并让现有 `ExitRequested` 优雅停止服务；
- 若明确退出流程已进入允许关闭阶段，则放行关闭，不再隐藏；
- 其它窗口事件继续保留现有焦点恢复逻辑。

不要用“窗口是否可见”推断是否允许退出。隐藏窗口、窗口已经关闭、系统关机和托盘退出可能产生不同事件顺序，必须使用显式、线程安全的退出状态。

关闭行为偏好也不能在 `CloseRequested` 中临时从前端查询。前端设置完成后，shell 才是关闭事件的权威状态源；前端 WebView 已隐藏或正在关闭时，不应再依赖 IPC 请求决定是否退出。

### 4. 保留并重构退出收敛

现有 `ExitRequested` 逻辑是正确的资源清理边界，应继续由它负责：

- 阻止第一次退出请求；
- 等待本地服务最多 5 秒优雅停止；
- 再显式退出应用；
- 防止显式退出再次触发时重复启动清理任务。

建议把原本单一的 `EXIT_REQUESTED` 语义拆成类似以下状态：

```text
close_is_allowed: 是否允许本次 CloseRequested 真正关闭窗口
exit_started:     是否已经启动应用退出清理
```

状态只由 desktop shell 修改，托盘模块通过受控函数请求退出，不直接写入原子变量。这样既能阻止普通关闭，又能让明确退出沿用已有异步停服流程。

### 5. 在 setup 中创建托盘

主窗口创建成功后初始化托盘。初始化失败必须有明确策略：

- Windows 首发版本：托盘是“关闭转后台”功能的必要依赖，建议初始化失败时阻止启用关闭拦截，并记录可诊断错误；不应让窗口被隐藏后没有恢复入口；
- macOS/Linux：先完成各桌面环境的图标、菜单和点击事件 smoke，再将关闭转后台作为正式支持行为；
- 如果产品决定托盘是所有平台的硬要求，也可以在 setup 失败时直接阻止启动，但需要提供清晰的系统级错误，而不是静默退出。

首选的兼容策略是由 shell 保存一个“托盘可用”状态：可用时拦截关闭，不可用时保持默认关闭行为。该状态不需要进入 Shell Bridge 或前端运行快照。

如果用户选择了“最小化到系统托盘”但托盘初始化失败，必须采用可恢复的 fail-safe 行为：本次关闭直接按“退出应用”处理，不能隐藏主窗口后留下没有恢复入口的进程。偏好值仍可保留，待下次托盘可用时继续生效，并通过 Notice 或偏好设置状态提示用户。

## 与现有启动和运行配置的关系

### 首屏隐藏

当前主窗口在前端首帧就绪前隐藏，属于加载防闪烁机制；托盘后台行为是运行时关闭窗口后的长期状态，二者必须区分：

- 首次启动正常由 `frontendReady` 显示；启动门卫未完成时不显示空白窗口，而由原生壳提示并退出；
- 托盘图标可以在 shell setup 后创建；
- 用户在首帧完成前主动点击托盘时，可以按用户明确意图显示窗口；
- 托盘恢复不能绕过前端已有的配置/错误状态，也不能直接打开 Axum 地址。

### 单实例

二次启动不会打开新进程。`tauri-plugin-single-instance` 回调应继续调用统一恢复函数，使“窗口已隐藏”和“窗口已最小化”都恢复到首个实例。二次启动参数、工作目录和 URL 继续按现有文档忽略。

### server-mode

隐藏窗口不影响 server-mode：

- 本地服务继续监听；
- 其它设备继续通过已发布的局域网地址访问；
- 防火墙规则保持现有持久化策略；
- 关闭窗口不触发防火墙清理；
- 只有托盘退出或系统真正退出时才停止服务。

托盘菜单文案应明确“退出”而不是“关闭窗口”，避免用户误以为关闭窗口会终止 server-mode。

### 远端/本机模式

client-only、connect-to-remote 和 self-hosted 也采用相同的窗口后台行为。隐藏窗口不改变模式、不清除认证状态；托盘恢复后继续使用当前前端会话。

偏好修改即时影响后续窗口关闭请求，不需要重启应用。修改偏好本身不应启动或停止本地服务；只有之后实际选择“退出应用”并关闭窗口时，才会执行退出清理。

## 权限、资源和依赖

- Tauri v2 托盘 API 属于 desktop shell，不需要增加通用 shell 权限；
- `capabilities/main.json` 不应新增任意 command，托盘事件在 Rust shell 内部处理；
- 不新增 `windows-sys`、PowerShell 或平台命令行依赖；
- 优先复用现有 `icons/icon.ico`、`icons/icon.icns` 和默认窗口图标；
- 若增加 macOS 模板图标，更新 `desktop/docs` 和图标资源说明；
- `tauri.conf.json` 的窗口配置只负责初始窗口属性，关闭转隐藏应在 Rust 的 `CloseRequested` 中实现，不通过前端拦截系统关闭按钮。

## 验收矩阵

### 自动化检查

- `cargo fmt --check`；
- `cargo test -p winestock-desktop`；
- `cargo check -p winestock-desktop`；
- Windows 发布构建 `pnpm desktop:build`；
- `git diff --check`。

纯 Rust 的状态辅助函数应覆盖：普通关闭被拦截、明确退出允许关闭、重复退出请求不重复启动清理、窗口恢复幂等。Tauri 事件对象和真实系统托盘行为仍需实机验证。

### Windows 实机

1. 启动应用，主窗口正常显示且托盘图标存在。
2. 点击窗口关闭按钮，窗口消失，进程仍存在，托盘图标仍存在。
3. server-mode 下关闭窗口后，从其它设备访问当前端口仍成功；本机服务没有停止或重启。
4. 点击托盘图标，窗口显示、取消最小化并获得焦点；重复点击不创建新窗口。
5. 右键托盘选择“打开”，结果与左键一致。
6. 右键托盘选择“退出”，本地服务释放端口，进程退出，防火墙规则不因退出被删除。
7. 窗口隐藏后再次启动程序，只恢复首个实例，不创建第二个进程。
8. 前端未发送 `frontendReady` 时，启动超时会显示原生错误提示并退出，不把未初始化窗口交给用户。
9. 托盘初始化失败的模拟路径不会把窗口置于无法恢复的隐藏状态。
10. 偏好设置默认是“最小化到系统托盘”，关闭窗口后进程和 server-mode 服务仍在运行。
11. 切换为“退出应用”后关闭窗口，进程退出并释放本地服务端口；再次打开应用后偏好仍保持。
12. server-mode 下从“最小化到系统托盘”切换为“退出应用”时，提示会说明关闭窗口将停止对外服务。
13. 托盘菜单“退出”不受偏好值影响，在两种偏好下都执行完整退出流程。

### macOS/Linux

- 目标平台可以编译 desktop crate；
- 常见桌面环境可以显示托盘/状态栏图标；
- 左键恢复和显式退出事件顺序正确；
- macOS 深色/浅色菜单栏下图标可读；
- Linux 在不提供托盘支持的桌面环境下按既定 fallback 行为处理，不宣称所有环境都支持。

## 不在本次实现范围

- 不在本托盘方案中实现开机启动或启动时静默，相关设计见 [`desktop-autostart.md`](desktop-autostart.md)；
- 不新增前端“关闭应用”页面或第二套原生设置 UI；
- 不改变 runtime 配置保存、认证、数据库和防火墙规则语义；
- 不把托盘状态加入业务 API 或 Shell Bridge；
- 不把关闭窗口转后台扩展为系统休眠、服务进程拆分或后台通知中心；
- 不照搬 KeyWine 的标题栏、通知、应用设置或业务组件。

## 实施顺序

1. 增加并测试 desktop 内部的退出状态辅助函数，先保证普通关闭与明确退出可区分。
2. 增加 desktop preferences 模型、持久化和具名 Shell Bridge command，接入前端偏好设置。
3. 新增托盘模块，复用现有应用图标和 `show_main_window`。
4. 接入 `CloseRequested`、托盘恢复和托盘退出，保持 `ExitRequested` 的服务清理边界。
5. 完成 Windows 实机验证，再补充 macOS/Linux 构建与托盘能力验证。
6. 实现完成后更新 `docs/platforms.md`、`docs/code-map/desktop.md`、Shell Bridge 文档和本目录 README 的当前状态说明，并提交对应代码与文档。

## API 依据

本方案依据当前 Tauri v2 文档中的 `TrayIconBuilder`、`on_tray_icon_event`、`on_menu_event`、`WebviewWindow::show`、`unminimize`、`set_focus` 和退出事件处理方式整理。实现时以锁定版本 `tauri = 2.11.5` 的编译结果为准：

- Tauri system tray：<https://v2.tauri.app/learn/system-tray/>；
- Tauri v2 事件迁移说明：<https://v2.tauri.app/start/migrate/from-tauri-1/>；
- 当前仓库 Tauri 版本：[`Cargo.toml`](../../../Cargo.toml)。
