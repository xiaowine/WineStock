# Desktop 启动门卫提示整改报告

> 文档状态：已实施
> 涉及组件：`desktop`、`frontend`
> 编制日期：2026-08-02
> 说明：本文中的“门卫”指 Desktop 启动阶段的 WebView2 版本门卫和 Shell Bridge 加载/握手门卫，不涉及运行设置中的端口或
> Windows 防火墙提示。

## 1. 背景与目标

Desktop 在创建可用界面前有两道必要门卫：

1. 检查 Windows WebView2 Runtime 是否存在且达到最低版本；
2. 检查前端是否能够读取并校验 Shell Bridge，完成事件订阅、平台扩展校验和首屏就绪握手。

这两类故障都属于应用启动基础设施故障，不是运行配置、HTTP 服务、鉴权或业务错误。当前实现已经具备阻断能力，
但用户看到的提示不能判断问题发生在哪一步，也不知道应重新安装 WineStock、重新启动还是联系维护人员。

本次整改目标：

- 明确区分 WebView2 运行时故障和 Shell Bridge 故障；
- 区分可由用户修复的原因与只能重新安装/升级的原因；
- 原生提示给出下一步动作，前端不在桥不可用时承担错误展示；
- 保留安全的稳定错误码和诊断日志，不把路径、HRESULT 或内部堆栈直接展示给用户；
- 保留面向用户的统一易懂文案，并在正文末尾追加受控的 `错误代码：...`，便于根据截图或反馈定位具体失败原因；
- 避免启动超时兜底把空白或半初始化 WebView 当成可用页面展示。

## 2. 当前实现路径

### 2.1 WebView2 版本门卫

当前路径：

```text
desktop/src/main.rs::main
  -> webview_compatibility::check()
  -> GetAvailableCoreWebView2BrowserVersionString
  -> 区分缺失、版本格式异常、版本过低和检查 API 失败
  -> 与 M111（111.0.0.0）比较
  -> 不支持：按稳定原因显示 rfd 原生错误对话框
  -> 进程退出
```

实现位置：

- [`desktop/src/webview_compatibility.rs`](../../src/webview_compatibility.rs)：调用 WebView2 Loader API，读取并比较版本；
- [`desktop/src/main.rs`](../../src/main.rs)：在 app data、RuntimeManager 和主窗口创建前执行门卫。

整改前失败提示固定为：

```text
标题：WineStock 无法启动
内容：WineStock 依赖损坏，请重新安装软件后重试。
```

整改前问题：

- 未安装 WebView2、版本低于 M111、版本字符串无法解析和 Loader API 读取失败全部显示同一文案；
- “依赖损坏”容易让用户误以为 WineStock 文件损坏，实际可能只是 WebView2 未安装或版本过低；
- 虽然最终恢复动作应统一为重新安装 WineStock，但当前文案没有说明安装器会补全 WebView2，也没有指出具体是 WebView2 运行组件问题；
- 没有显示当前版本、最低要求或检查失败类型，客服和测试无法根据截图判断问题；
- Debug 的 `force_webview_block` 复用生产提示，开发测试无法确认阻断来源。

### 2.2 Shell Bridge 加载与握手门卫

当前路径：

```text
frontend/src/main.ts::bootstrapFrontend
  -> initializeShellRuntime()
  -> createShellBridge()
  -> assertCompleteShellBridge()
  -> getRuntimeSnapshot()
  -> assertCompatibleRuntimeSnapshot()
  -> Desktop 扩展 repairFirewall 校验
  -> 订阅 runtime-state / app-resumed 事件
  -> Vue 挂载、原生返回订阅
  -> frontendReady()
```

失败路径：

```text
任一初始化/契约/订阅/就绪调用失败
  -> reportShellBridgeFailure(error, stableFailureCode)
  -> Tauri shell_frontend_failed
  -> 隐藏 WebView
  -> 原生侧按稳定原因映射 rfd 文案
  -> 退出进程
```

实现位置：

- [`frontend/src/main.ts`](../../../frontend/src/main.ts)：启动编排、Bridge 初始化、Vue 挂载和 `frontendReady`；
- [`frontend/src/shell/runtime.ts`](../../../frontend/src/shell/runtime.ts)：Bridge 初始化、契约校验和失败上报；
- [`frontend/src/shell/contract.ts`](../../../frontend/src/shell/contract.ts)：快照、公共方法和 Desktop 扩展校验；
- [`desktop/src/commands.rs`](../../src/commands.rs)：`shell_frontend_failed` 原生失败处理；
- [`desktop/src/main.rs`](../../src/main.rs)：隐藏窗口、首帧显示和 8 秒原生超时兜底。

整改前失败提示固定为：

```text
标题：WineStock 无法加载
内容：加载异常，请更新后重试。
```

整改前问题：

- `shell_frontend_failed` 只接收自由文本，无法保证失败原因稳定、可分类；
- Bridge 不可用、快照不兼容、方法缺失、事件订阅失败、原生扩展不一致和 `frontendReady` 调用失败全部使用同一文案；
- “请更新”不是所有 Bridge 故障的正确处理方式，很多情况重新启动即可，另一些情况才需要修复或重新安装；
- 8 秒超时只检查 `FRONTEND_READY`，随后显示隐藏窗口。如果前端初始化失败且无法成功上报，用户可能看到空白或半初始化界面；
- 原生对话框只有“确定”，没有清晰的恢复动作，也没有告诉用户应用会退出；
- Bridge 门卫失败和 WebView2 门卫失败的视觉标题相近，用户无法区分是系统运行时问题还是应用桥加载问题。

## 3. 整改原则

### 3.1 两道门卫必须使用不同的故障域

| 故障域              | 责任组件            | 用户应该理解的事实                                 | 首选动作                                      |
|------------------|-----------------|-------------------------------------------|-------------------------------------------|
| WebView2 Runtime | Desktop 原生壳     | Windows 缺少或不满足 WineStock 所需的 WebView2 运行时 | 重新安装 WineStock，由安装器补全或更新 WebView2 Runtime |
| Shell Bridge     | Desktop 与前端启动链路 | 应用界面无法连接 Desktop 运行组件，不能安全进入业务页面          | 重新启动；持续失败时修复或重新安装 WineStock               |

两者都可以在原生对话框中处理，但标题、稳定错误码、文案和诊断日志必须不同。

### 3.2 提示必须说明“发生了什么”和“下一步做什么”

提示不需要暴露 COM 类型、Tauri command 名称、完整路径或堆栈，但至少要包含：

- 当前检查失败的对象；
- 对用户可理解的原因；
- 应执行的恢复动作；
- 点击确认后应用是否退出。

### 3.3 门卫失败不能进入业务 UI

Bridge 契约未完成时，前端不能依赖 Notice、运行设置页或服务错误页展示提示，因为这些页面本身依赖 Bridge 初始化。
原生壳应保持窗口隐藏，显示原生错误对话框并退出。只有已经完成 Bridge 初始化且单纯业务服务不可用时，才进入前端恢复页面。

## 4. 已采用的稳定状态与文案

### 4.1 WebView2 门卫

当前 `WebViewRuntimeInfo` 使用内部原因，不把底层类型直接暴露给前端：

```text
webview2_missing
webview2_version_too_old
webview2_version_invalid
webview2_version_check_failed
webview2_forced_block（仅 Debug）
```

当前原生提示：

| 状态         | 标题                  | 内容                                                                      | 用户动作           |
|------------|---------------------|-------------------------------------------------------------------------|----------------|
| 未安装/无法找到   | 无法启动 WineStock      | 未检测到 WineStock 所需的 WebView2 Runtime。请重新安装 WineStock，安装器会补全所需组件。确认后 WineStock 将退出。       | 重新安装 WineStock |
| 版本过低       | WineStock 运行组件版本过低  | 当前 WebView2 版本低于 WineStock 的最低要求（M111）。请重新安装 WineStock，安装器会补全满足要求的组件。确认后 WineStock 将退出。 | 重新安装 WineStock |
| 版本格式异常     | 无法检查 WineStock 运行组件 | 无法正确读取 WebView2 Runtime 版本。请重新安装 WineStock，安装器会重新配置所需组件。确认后 WineStock 将退出。                | 重新安装 WineStock |
| 检查 API 失败  | 无法检查 WineStock 运行组件 | WineStock 无法确认 WebView2 Runtime 是否可用。请重新安装 WineStock，安装器会重新配置所需组件。确认后 WineStock 将退出。      | 重新安装 WineStock |
| Debug 强制阻断 | WebView2 门卫测试       | 当前为 Debug 测试配置，已模拟 WebView2 版本不满足要求。确认后 WineStock 将退出。                                    | 确认并退出          |

版本字符串可以写入诊断日志；用户提示只显示 M111，不显示完整版本号，除非后续增加“复制诊断信息”入口。

用户可见诊断码与内部原因码分开维护：

```text
WEBVIEW2_MISSING
WEBVIEW2_VERSION_TOO_OLD
WEBVIEW2_VERSION_INVALID
WEBVIEW2_VERSION_CHECK_FAILED
WEBVIEW2_FORCED_BLOCK（仅 Debug）
```

每条原生提示正文最后追加一行，例如：

```text
未检测到 WineStock 所需的 WebView2 Runtime。请重新安装 WineStock，安装器会补全所需组件。确认后 WineStock 将退出。
错误代码：WEBVIEW2_MISSING
```

### 4.2 Shell Bridge 门卫

Desktop 内部使用以下稳定失败类型：

```text
shell_bridge_unavailable
shell_bridge_snapshot_invalid
shell_bridge_version_mismatch
shell_bridge_method_missing
shell_bridge_extension_invalid
shell_bridge_event_subscription_failed
shell_bridge_ready_failed
frontend_load_timeout
```

当前原生提示：

| 状态           | 标题                 | 内容                                             | 用户动作  |
|--------------|--------------------|------------------------------------------------|-------|
| Bridge 不可用   | WineStock 无法连接桌面组件 | 桌面运行组件没有正常响应。请重新启动 WineStock；问题仍存在时请修复或重新安装软件。确认后 WineStock 将退出。 | 确认并退出 |
| 快照/契约无效      | WineStock 无法加载     | 当前版本的界面与桌面运行组件不匹配。请重新安装同一版本的 WineStock。确认后 WineStock 将退出。        | 确认并退出 |
| Desktop 扩展缺失 | WineStock 无法加载     | 桌面运行组件缺少必要能力。请重新安装软件。确认后 WineStock 将退出。                          | 确认并退出 |
| 事件/握手失败      | WineStock 页面加载失败   | WineStock 页面无法完成启动握手。请重新启动软件；问题仍存在时请修复或重新安装。确认后 WineStock 将退出。   | 确认并退出 |
| 首屏超时         | WineStock 页面加载超时   | 页面未能在规定时间内完成加载。请重新启动软件；问题仍存在时请重新安装软件。确认后 WineStock 将退出。        | 确认并退出 |

“更新后重试”不应作为所有 Bridge 错误的默认文案。Bridge 契约版本不匹配时，应建议重新安装同一版本的 WineStock；
WebView2 的所有检查失败则统一建议重新安装 WineStock，由安装器负责补全组件。

用户可见诊断码映射如下：

| 内部失败码 | 用户可见错误码 |
|---|---|
| `shell_bridge_unavailable` | `SHELL_BRIDGE_UNAVAILABLE` |
| `shell_bridge_snapshot_invalid` | `SHELL_BRIDGE_SNAPSHOT_INVALID` |
| `shell_bridge_version_mismatch` | `SHELL_BRIDGE_VERSION_MISMATCH` |
| `shell_bridge_method_missing` | `SHELL_BRIDGE_METHOD_MISSING` |
| `shell_bridge_extension_invalid` | `SHELL_BRIDGE_EXTENSION_INVALID` |
| `shell_bridge_event_subscription_failed` | `SHELL_BRIDGE_EVENT_SUBSCRIPTION_FAILED` |
| `shell_bridge_ready_failed` | `SHELL_BRIDGE_READY_FAILED` |
| `frontend_load_timeout` | `FRONTEND_LOAD_TIMEOUT` |

Desktop 原生提示正文最后追加对应的 `错误代码：...`。未知或非法的前端值必须降级为
`SHELL_BRIDGE_UNAVAILABLE`，不能将原始字符串拼进用户文案。

## 5. 已实施的实现调整

### 5.1 WebView2 检查

1. `webview_compatibility::check()` 返回 `reason` 或等价的内部枚举，而不是只返回 `version` 和 `supported`。
2. `main.rs` 根据原因选择稳定的原生提示文案，但所有 WebView2 原因统一指向“重新安装 WineStock”。
3. 对缺失、过低、格式异常和 API 读取失败分别记录结构化日志。
4. 保持检查时机不变：在创建主窗口和启动本地服务前完成；不将 WebView2 检查下沉到前端。
5. Release 仍使用 M111 作为最低主版本，Debug 强制阻断只用于测试，不与真实 WebView2 故障混淆。

### 5.2 Bridge 失败上报

1. 前端只上报稳定的失败类别和安全摘要，不能把任意异常文本直接作为 UI 文案。
2. `shell_frontend_failed` 接收稳定失败代码；原生侧按稳定类别映射用户文案，并将稳定原因写入受控日志。
3. `assertCompatibleRuntimeSnapshot`、`assertCompleteShellBridge` 和 Desktop 扩展校验分别映射到不同类别。
4. `frontendReady` 调用失败和事件订阅失败归入 Bridge 握手故障，不显示运行设置或业务错误。
5. 失败上报成功后保持窗口隐藏并退出；失败上报本身失败时，原生壳仍应有统一的超时错误路径。

### 5.3 启动超时

当前 8 秒兜底采用单一安全路径：

- `frontendReady` 已到达：保持正常显示/静默启动逻辑；
- 已经收到 `shell_frontend_failed`：保持窗口隐藏，等待原生提示并退出；
- 8 秒内既未收到 `frontendReady` 也未收到失败上报：由原生壳显示“页面加载超时”提示并退出。

超时路径不再显示任意半初始化 WebView，避免空白窗口被误认为可用页面。

### 5.4 诊断信息

当前日志记录：

```text
gate = webview2 | shell_bridge
reason = 稳定原因码
webview_version = 可选，只有 WebView2 门卫记录
debug = true/false
```

不得记录：数据库路径、认证 token、密码、完整命令行参数或 WebView 页面中的业务数据。

### 5.5 用户可见错误码

错误码是供测试、客服和开发人员定位的稳定标识，不承担面向用户解释技术细节的职责。Desktop 与 Android
可以使用相同的用户文案，但必须在正文最后以独立换行追加错误码；不要新增独立的大型错误面板，也不要把
HRESULT、堆栈、路径或任意异常文本展示给用户。平台接收前端错误码时只允许映射表中的值通过，未知值统一使用
通用错误码。

## 6. 不建议的调整

- 不把 WebView2 门卫改成前端检查；WebView 未创建或无法执行 JavaScript 时前端无法可靠报告结果。
- 不让前端在 Bridge 未初始化时显示 Notice 作为唯一错误反馈。
- 不把 Shell Bridge 故障误归为 WebView2 故障；WebView2 的不同内部原因可以统一使用重新安装 WineStock 的恢复路径。
- 不因为 Bridge 失败而启动本地 core；启动门卫失败时业务服务也应保持未启动。
- 不让 8 秒兜底覆盖已经确认的 Bridge 失败状态。
- 不在提示中显示 HRESULT、Tauri command、Rust 模块名或内部绝对路径。

## 7. 验收矩阵

### WebView2 门卫

- 未安装 WebView2：显示“未检测到 WebView2 Runtime”，不显示“端口”“服务启动失败”等业务提示。
- WebView2 主版本低于 M111：明确显示运行组件版本过低，并统一指向重新安装 WineStock，不要求用户单独更新 WebView2。
- 版本字符串异常或 Loader API 失败：显示无法检查，并统一要求重新安装 WineStock。
- 门卫失败时不创建主窗口、不加载前端、不启动本地 core。
- Debug 强制阻断与真实缺失提示可以区分，测试日志包含强制阻断标识。

### Shell Bridge 门卫

- `getRuntimeSnapshot` 失败：显示桌面组件无响应，窗口不进入业务界面。
- 快照字段不兼容：显示版本不匹配，建议重新安装同一版本。
- 必要方法缺失或 Desktop 扩展缺失：显示桌面组件缺失，不显示运行配置错误。
- 事件订阅失败或 `frontendReady` 失败：显示页面加载/启动握手失败。
- Bridge 失败上报成功后不再被 8 秒兜底显示空白窗口覆盖。
- 正常启动完成后仅在 `frontendReady` 之后显示主窗口。
- 所有失败均有稳定原因码和受控日志，用户提示不泄漏内部错误详情。
- Desktop 和 Android 的启动失败正文末尾均包含稳定的 `错误代码：...`；未知 Bridge 错误码降级为通用码，
  不显示原始输入。

### 回归验证

- Desktop Debug：正常启动、强制 WebView2 阻断、Bridge 契约故障模拟。
- Windows 10/11：WebView2 未安装、低版本、正常版本三种环境。
- 首次无配置、已有本地配置、远端配置和服务启动失败场景不受影响。
- `cargo test -p winestock-desktop`。
- `cd frontend && pnpm exec vue-tsc -b`。
- `cd frontend && pnpm run build:desktop`。
- `git diff --check`。

## 8. 实施结果

本次整改已完成：

- WebView2 门卫增加 `Missing`、`VersionTooOld`、`VersionInvalid`、`VersionCheckFailed` 和 Debug `ForcedBlock` 内部原因；
  所有真实 WebView2 故障统一要求重新安装 WineStock，由安装器补全或更新 WebView2。
- Shell Bridge 通过稳定失败代码上报，Desktop 原生层按“组件无响应、版本/快照不匹配、能力缺失、握手失败”选择提示，
  不再展示前端异常文本。
- 前端初始化、事件订阅和 `frontendReady` 失败均保持窗口隐藏；8 秒未完成握手时由原生壳提示加载超时并退出。
- Web/Android transport 保持 `reportFrontendFailure` 的兼容入口，Android 仍使用原有兼容页处理，不改变其用户流程。
- Desktop WebView2、Desktop Shell Bridge 和 Android WebView/Bridge 兼容页均追加受控的用户可见错误码；内部小写
  协议码与大写展示码分离，未知值统一降级为通用码。

## 9. 结论

门卫整改已完成。WebView2 的所有真实检查失败均由原生壳分类并要求重新安装 WineStock，
由安装器负责补全或更新运行组件；Shell Bridge 的初始化、契约、扩展、事件和握手失败均使用稳定原因码、
安全原生提示和退出流程；启动超时不会再展示空白窗口。

当前 Windows 11（Build 26200）开发环境已完成以下 smoke：Release 正常启动并显示 `WineStock` 主窗口；Release 传入 Debug
故障注入参数仍正常启动，确认 Release 忽略测试参数；Debug `--winestock-force-webview-block` 显示
`WebView2 门卫测试`，Debug Shell Bridge 安装失败和首屏握手失败参数均进入对应的受控失败流程。
仍需在目标 Windows 10/11 环境使用安装包覆盖 WebView2 缺失、低版本和正常版本矩阵；Android 的对应 JVM/APK
验证不属于 Desktop 门卫本地验收范围。
