# Desktop WebView 空闲回收方案

> 状态：已实施；Rust/前端自动化验证、Windows 发布构建和 Windows 同机托盘/WebView smoke 已通过；跨设备 server-mode 验证待具备第二台设备
>
> 日期：2026-08-03
>
> 范围：`desktop` Tauri v2 Shell + 共享 `frontend` Desktop 偏好

实施代码已按本文落地。Windows 同机已经实测真实托盘点击、WebView 销毁/重建、Axum 持续运行、单实例恢复、直接退出
和回收前后 WebView2 进程内存；跨设备 server-mode 访问仍需第二台设备，不能由同机访问替代。

## 1. 背景与结论

Desktop 当前点击关闭按钮后会隐藏主 `WebviewWindow`，进程、系统托盘、本地 Axum
服务和 WebView 页面都继续存在。隐藏只能避免窗口显示，不能释放页面的 JavaScript、Vue
组件、渲染器和请求上下文占用的内存。

本方案增加可选的 WebView 空闲回收能力：窗口隐藏到托盘后，若超过用户设置的空闲时间仍未
恢复，则销毁主 WebView；Tauri 进程、托盘和 `DesktopRuntimeManager` 继续运行。用户下一次
打开托盘或启动第二个实例时，Shell 重新创建主 WebView 并加载打包的前端资源。

Axum 服务始终保持运行，不因 WebView 隐藏、回收或恢复而停止、重启、换端口或重新初始化。
只有用户明确退出应用、系统退出或进程终止时，现有的 Axum 优雅关闭流程才执行。

推荐初始默认值：

- `webviewReclaimEnabled = false`，保持现有用户升级后的行为不变；
- `webviewReclaimIdleMinutes = 30`；
- 开启后可选择 `5 / 15 / 30 / 60 / 120 / 240` 分钟。

默认值可以在产品确认后改为开启，但不改变实现边界。

## 2. 目标与非目标

### 目标

1. 在现有前端 Desktop 偏好设置中提供 WebView 回收开关。
2. 开启后允许设置托盘隐藏期间的空闲回收时间。
3. 到期只销毁主 WebView，不停止 Axum、不改变 API 地址、不清理业务数据库和文件。
4. 托盘点击、托盘“打开”和单实例恢复都能在 WebView 已销毁时重新创建窗口。
5. 重新加载前端时沿用现有 Shell Bridge、运行快照、鉴权恢复和 `frontendReady` 启动流程。
6. 回收任务可取消、不会重复执行，也不会误触发应用退出。

### 非目标

- 不修改 `core`、HTTP API、OpenAPI 或数据库。
- 不停止或重启 `RunningLocalService`。
- 不新增 Axum 静态资源服务。
- 不把 WebView 回收设置同步到 Android、Server Shell 或远端服务。
- 不增加原生设置窗口；设置仍由共享前端的偏好 Dialog 拥有。
- 不持久化“已经隐藏了多久”的计时状态。进程重启后重新计时即可。

## 3. 所有权与依赖边界

```text
frontend
  -> 读取/编辑 DesktopPreferences
  -> 通过现有 Shell Bridge 保存偏好

desktop Shell
  -> 持久化 DesktopPreferences
  -> 维护 WebViewWindow 生命周期和空闲计时器
  -> 创建、销毁、恢复主 WebView
  -> 保持 DesktopRuntimeManager 和 Axum 服务运行

core/shared
  -> 无改动
```

WebView 生命周期属于 `desktop`，不能放入 `core`、`shared` 或前端运行时模块。前端只表达
用户偏好，不直接调用 Tauri 窗口 API，也不推断 WebView 是否已经被销毁。

## 4. 偏好契约

复用现有的 `getDesktopPreferences` / `setDesktopPreferences` Shell Bridge 扩展，不增加新
command。Desktop 偏好增加两个字段：

```ts
interface DesktopPreferences {
  version: 1;
  closeBehavior: "minimize-to-tray" | "exit-application";
  autostartEnabled: boolean;
  autostartSilent: boolean;
  webviewReclaimEnabled: boolean;
  webviewReclaimIdleMinutes: 5 | 15 | 30 | 60 | 120 | 240;
}
```

### 字段语义

| 字段 | 所有者 | 默认值 | 语义 |
| --- | --- | --- | --- |
| `webviewReclaimEnabled` | Desktop Shell + Desktop 前端 | `false` | 托盘隐藏后是否允许自动销毁主 WebView |
| `webviewReclaimIdleMinutes` | Desktop Shell + Desktop 前端 | `30` | 隐藏且未恢复时等待的分钟数 |

字段继续属于 `desktop-preferences.json`，不进入共享 `AppConfig`。保持偏好版本为 `1`，新字段
使用 serde 默认值兼容已有文件。已有文件缺少字段时自动补默认值；未知版本继续按当前逻辑回退
到默认偏好。

Shell 必须在保存时做权威校验：

- `webviewReclaimIdleMinutes` 只能是允许的预设值；
- 无效值返回稳定的 `desktop_preferences_invalid` 错误；
- 失败时不能修改内存状态；
- 前端只有收到保存成功后的返回值才更新本地显示状态。

如果将来需要任意分钟数，应先扩展协议为 `1..1440` 的整数并同步 UI、校验和测试；本次不采用
任意数字，避免 `0`、极短时间和超长时间造成难以解释的生命周期行为。

## 5. 前端偏好设置设计

### 入口

沿用 `AppPreferencesDialog.vue` 的现有“窗口”设置区域，不新增页面和原生 Dialog。
Desktop Shell Bridge 不可用时，整个 WebView 回收设置隐藏；Web、Android 继续不显示该 Desktop
专属设置。

### 控件

在现有“关闭窗口时”设置之后增加：

- 二态开关：`空闲时回收 WebView`；
- 选择控件：`回收等待时间`，选项为 `5 分钟、15 分钟、30 分钟、1 小时、2 小时、4 小时`；
- 开关关闭时，等待时间选择控件禁用但保留已保存的值。

使用已有偏好设置控件、表单字段、Select/Toggle 样式和 Notice 反馈，不创建新的视觉语言。
选择控件使用稳定宽度，长文案在移动宽度下允许换行，不产生横向溢出。

### 保存行为

- 开关和时间选择沿用当前 Desktop 偏好的即时保存方式；
- 保存失败时恢复上一次成功值，并使用现有 Notice 展示错误；
- 开关关闭时取消未来的回收计划；
- 开关开启或修改时长只影响后续隐藏事件，当前可见窗口不销毁、不重载；
- 不在前端显示实时倒计时，避免把后台生命周期变成持续变化的页面状态。

### 平台兼容

`frontend/src/shell/contract.ts` 的运行时断言必须校验两个新字段。Web fallback 和 Android
未提供 Desktop 偏好扩展，因此不需要伪造字段；`getDesktopPreferences()` 返回 `null` 时不
渲染该区域。

## 6. Desktop 生命周期模型

### WebView 状态

在现有 `AppLifecycleState` 中增加明确的 WebView 状态，或新增一个由 Desktop Shell 独占的
窗口生命周期状态对象：

```text
Alive       主 WebView 存在且可使用
Hidden      主 WebView 存在但窗口已隐藏
Disposing   已决定销毁，正在处理窗口事件
Disposed    主 WebView 不存在，进程和托盘仍在
Restoring   正在重新创建并等待前端握手
```

不能通过“是否可见”推导状态。隐藏、销毁、应用退出和托盘恢复的事件顺序不同，必须使用
显式状态和 generation 标识。

### 应用退出状态

保留现有两个退出语义，并增加 WebView 回收语义：

```text
close_allowed             是否允许本次窗口关闭事件真正关闭
exit_started              是否已经启动应用退出清理
webview_dispose_started   是否正在执行 WebView 回收
webview_generation        当前主 WebView 的代次
```

`webview_dispose_started` 绝不能复用 `close_allowed` 或 `exit_started`。WebView 销毁可能产生
窗口关闭/应用退出相关事件；这些事件必须被识别为“回收窗口但继续运行”，不能进入 Axum 停止
和 `handle.exit(0)` 流程。

## 7. 正常运行流程

### 7.1 窗口关闭到托盘

```text
CloseRequested
  -> closeBehavior == minimize-to-tray 且托盘可用
  -> api.prevent_close()
  -> window.hide()
  -> 状态 Alive -> Hidden
  -> 若 webviewReclaimEnabled，启动当前 generation 的空闲计时器
```

以下情况不启动回收计时器：

- `closeBehavior == exit-application`；
- 托盘不可用；
- 应用退出已经开始；
- WebView 已经处于 `Disposing` 或 `Disposed`；
- 用户关闭请求不是主窗口。

隐藏后不调用 `DesktopRuntimeManager`，Axum 继续监听原来的实际端口。

### 7.2 计时器取消

计时器使用单调时间和 generation/cancellation token。以下任一事件发生时取消当前计时器：

- 托盘左键恢复；
- 托盘菜单“打开”；
- 单实例插件通知已有进程恢复；
- 窗口被其它受控路径显示；
- 用户关闭回收开关；
- 应用进入退出流程。

计时器触发时必须再次检查：

1. generation 仍然相同；
2. 回收开关仍然开启；
3. 窗口仍然存在并且隐藏；
4. `closeBehavior` 仍为 `minimize-to-tray`；
5. 没有进入应用退出流程。

任一检查失败都直接结束任务，不销毁窗口。

Windows smoke 可以在启动参数中临时覆盖本次进程的等待时间，避免测试必须等待完整的分钟预设：

```text
winestock-desktop.exe --winestock-webview-reclaim-idle-seconds=10
```

也支持将秒数作为下一个参数传入。该值只接受 `1..86400` 秒，不写入
`desktop-preferences.json`，且仍要求 `webviewReclaimEnabled = true`；未传入时继续使用前端偏好中的分钟值。

### 7.3 到期销毁

```text
计时器到期
  -> 标记 webview_dispose_started
  -> 状态 Hidden -> Disposing
  -> 允许本次受控 destroy/close 事件继续
  -> 销毁 main WebviewWindow
  -> 状态 Disposing -> Disposed
  -> 保留 TrayIcon、AppHandle、DesktopRuntimeManager 和 Axum
```

回收动作不清理：

- 本地 Axum 监听端口；
- SQLite 和文件仓；
- `desktop-preferences.json`；
- WebView2 用户数据目录；
- 运行配置和 API 地址。

WebView2 的浏览器共享进程可能继续存在，验收时以进程内存和页面渲染进程释放情况为准，不能
承诺整个 WebView2 进程树归零。

### 7.4 托盘恢复

所有恢复入口统一调用 `ensure_main_window()`：

```text
ensure_main_window
  -> 取消回收计时器
  -> 若 main 存在：show + unminimize + set_focus
  -> 若 main 不存在：进入 Restoring
       -> WebviewWindowBuilder 创建 main
       -> 重新注入 runtime metadata
       -> 重新设置 WebView 隐私选项
       -> 加载 index.html
       -> 等待当前 generation 的 frontendReady
       -> 显示并聚焦窗口
       -> 状态 Restoring -> Alive
```

托盘点击、托盘“打开”、`tauri-plugin-single-instance` 回调和未来其它恢复入口必须复用同一
函数，不得各自复制窗口创建逻辑。

如果创建失败：

- 保留托盘和 Axum；
- 记录稳定诊断码；
- 不退出整个进程；
- 下一次托盘打开仍允许重试；
- 不把 Axum 服务错误误报成 WebView 加载错误。

## 8. 首屏握手与代次处理

当前 `FRONTEND_READY` 和 `FRONTEND_FAILURE_REPORTED` 是进程级状态。WebView 被销毁后重新
创建时必须重置或改造为按 generation 记录，否则旧 WebView 的 ready 状态会绕过新的超时和
失败处理。

推荐使用：

```text
webview_generation: AtomicU64 或受 Shell 状态保护的递增编号
frontend_ready_generation: Option<u64>
frontend_failure_generation: Option<u64>
```

每次创建 WebView 都递增 generation。`frontendReady` 和 `shell_frontend_failed` 只接受当前
generation 的页面调用；旧页面的迟到调用不能显示、覆盖或退出当前窗口。

现有 8 秒启动门卫改为按 generation 启动：

- 初次启动的超时任务只检查初次 generation；
- 回收后恢复的超时任务只检查恢复 generation；
- 旧任务发现 generation 不一致后立即退出；
- 恢复失败只隐藏当前 WebView，不停止 Axum。

前端重新加载后仍使用已有顺序：读取 Shell Bridge、获取运行快照、恢复 API client 和会话、
健康检查、发送 `frontendReady`。不新增业务 HTTP 请求。

## 9. 对现有 Desktop 偏好的兼容

当前偏好文件已有：

```json
{
  "version": 1,
  "closeBehavior": "minimize-to-tray",
  "autostartEnabled": false,
  "autostartSilent": true
}
```

实施后新增：

```json
{
  "version": 1,
  "closeBehavior": "minimize-to-tray",
  "autostartEnabled": false,
  "autostartSilent": true,
  "webviewReclaimEnabled": false,
  "webviewReclaimIdleMinutes": 30
}
```

旧文件缺少新字段时由 serde 默认值补齐。保存时写出完整字段。因为只是同一版本的向后
兼容新增字段，不升级偏好版本；偏好文档和测试必须明确这一点。

## 10. 代码实施拆分

### Desktop Rust

- `desktop/src/preferences.rs`
  - 扩展 `DesktopPreferences` 字段、默认值、serde 兼容和取值校验；
  - 增加旧 JSON 缺少字段的测试。

- `desktop/src/lifecycle.rs`
  - 增加 WebView 状态、generation 和回收/恢复状态辅助方法；
  - 增加不持久化的秒级 smoke 启动参数解析；
  - 保持应用退出状态与 WebView 回收状态独立。

- `desktop/src/window.rs`
  - 集中主窗口创建、恢复、销毁和 generation 初始化；
  - 不拥有 Axum 生命周期。

- `desktop/src/main.rs`
  - 初始启动和恢复启动都调用统一窗口创建函数；
  - 连接 `CloseRequested`、托盘恢复、前端握手超时和 WebView 回收状态；
  - `RunEvent::ExitRequested` 只在真实应用退出时关闭 Axum。

- `desktop/src/tray.rs`
  - 托盘恢复入口改为 `ensure_main_window()`；
  - 托盘退出仍调用现有完整退出流程。

- `desktop/src/commands.rs`
  - 偏好 command 增加服务端校验；
  - `frontendReady`、失败上报和超时逻辑改为 generation 感知。

- `desktop/src/lib.rs` 与代码地图/组件文档
  - 如新增独立生命周期模块，补充模块导出、中文文件头和代码地图说明。

### Frontend

- `frontend/src/shell/contract.ts`
  - 增加两个 DesktopPreferences 字段、默认值和运行时断言。

- `frontend/src/components/preferences/AppPreferencesDialog.vue`
  - 增加 WebView 回收开关和等待时间选择；
  - 沿用现有即时保存、失败回滚、Desktop 能力隐藏逻辑。

- `frontend/src/shell/runtime.ts` 与 Tauri transport
  - 保持现有 command 名称和传输方式；
  - 仅同步扩展后的 DTO，不增加新的原生能力。

- `frontend/tests/`
  - 增加偏好 DTO 断言和默认值/兼容性测试；
  - 如已有偏好组件测试入口，补充开关、禁用选择器和保存失败回滚测试。

### 文档

- 更新 `docs/shell-bridge.md` 的 DesktopPreferences 定义和 Desktop 生命周期说明；
- 更新 `docs/platforms.md`、`desktop/docs/README.md` 和 `docs/code-map/desktop.md` 的当前状态；
- 更新 `frontend/docs/README.md` 或相关偏好设置说明，明确 Desktop 专属设置不影响 Android/Web；
- 本方案实施完成后，将本文件状态改为已实施并补充实际验收结果；
- 更新 `docs/implementation-notes/README.md` 索引。

## 11. 测试与验收

### Rust 单元测试

- 新偏好默认值正确；
- 旧版本 JSON 缺少两个字段时正常加载；
- 非法回收时间被拒绝；
- 开关关闭时不创建回收任务；
- 窗口恢复会取消计时器；
- generation 变化后旧计时器不会销毁新窗口；
- WebView 回收不会设置 `exit_started`；
- 应用退出只启动一次 Axum 清理；
- 回收状态和退出状态互不污染。

### 前端测试

- Desktop 偏好 payload 包含新字段；
- 旧默认值可在缺少 Desktop 扩展时安全回退；
- WebView 回收关闭时等待时间控件禁用；
- 保存失败恢复上一份成功值；
- Android/Web 不渲染 Desktop WebView 设置。

### Windows 实机 smoke

1. [x] 关闭回收开关，关闭窗口到托盘 15 秒后 WebView 仍为 6 个且可恢复，未发生重载。
2. [x] 开启回收并使用 `--winestock-webview-reclaim-idle-seconds=10`，隐藏后约 8 秒目标 WebView2 进程从 6 个降为 0，WineStock 进程和托盘仍在。
3. [x] 回收后真实点击托盘图标，主窗口重新可见，WebView2 重新出现 6 个，健康接口仍返回 200。
4. [x] 回收后再次启动程序，第二实例约 10 秒内退出，原实例 PID 保持不变并恢复主窗口，未创建第二个长期进程。
5. [x] 回收等待期间 `http://127.0.0.1:55853/api/health` 持续返回 200。
6. [ ] `server-mode` 回收后的其它设备访问：当前环境没有第二台设备，未执行，不以同机访问冒充跨设备证据。
7. [x] 回收后通过托盘“退出 WineStock”，进程和目标 WebView2 进程退出，健康接口停止响应。
8. [x] `exit-application` 下关闭窗口直接退出，未进入 WebView 回收，Axum 随退出停止。
9. [x] 多轮隐藏/恢复和回收/恢复未出现重复窗口、重复长期进程或错误退出。
10. [ ] 恢复加载超时故障注入：未做完整实机故障注入；generation/恢复失败路径由 Rust 测试和代码门卫覆盖。
11. [x] 回收前目标 WebView2 进程树约 332.8 MB，回收后目标目录进程数为 0；WineStock 主进程约 35.9 MB，未承诺固定释放比例。

### 前端视觉验收

至少检查：

- `1440 x 900` 桌面偏好 Dialog；
- 接近 `768px` 的窄桌面视口；
- `390 x 844` 移动视口；
- 开关关闭、开启、保存中、保存失败和选择控件禁用状态；
- 无横向溢出、字段文字不重叠、控制台无新增错误或警告。

## 12. 实施顺序

1. 先扩展 Desktop 偏好模型和 Shell Bridge DTO，补齐兼容测试。
2. 实现可测试的 WebView 状态、generation 和回收计时器，不接真实窗口。
3. 抽取统一主窗口创建/恢复函数，接入初次启动、托盘和单实例入口。
4. 接入隐藏后的计时器和受控 WebView 销毁，确保 Axum 生命周期不变。
5. 实现前端偏好 UI，并接入现有保存/失败回滚模式。
6. 更新 Shell Bridge、平台、前端文档和代码地图。
7. 执行 Rust/前端自动测试、构建、Windows smoke 和内存对比。

## 13. 交付判定

满足以下条件后才算完成：

- WebView 可按偏好回收和恢复；
- Axum 在隐藏、回收、恢复期间始终运行；
- 真实退出时 Axum 仍能优雅关闭；
- 偏好文件兼容已有用户；
- Web/Android 不显示 Desktop 专属设置；
- 旧 WebView 的迟到事件不会影响新 WebView；
- Windows smoke 和内存观察结果已记录；
- 代码、文档和测试差异只包含本功能。

## 14. 技术依据

Tauri v2 当前托盘 API 使用 `TrayIconBuilder`、`on_menu_event` 和
`on_tray_icon_event`；窗口恢复使用 `show`、`unminimize` 和 `set_focus`；主窗口不存在时
通过 `WebviewWindowBuilder` 重新创建。实现时仍以仓库锁定的 Tauri 版本编译结果为准。

- Tauri system tray：<https://v2.tauri.app/learn/system-tray/>；
- Tauri v2 官方文档：<https://v2.tauri.app/>。
