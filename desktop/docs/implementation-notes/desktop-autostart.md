# Desktop 自启动与静默启动实现方案

## 结论

Desktop 增加两个本机偏好：

- **开机自启**：用户登录操作系统后自动启动 WineStock；
- **静默启动**：由开机自启触发时保持主窗口隐藏，只在系统托盘中运行。

这两个选项属于 Tauri Desktop Shell 的本机生命周期偏好，不进入共享运行配置、业务 API、数据库或
Android/Web 端。自启动由官方 `tauri-plugin-autostart` 负责注册和取消，前端继续通过现有 Shell Bridge
访问，不直接调用 Tauri 插件 API。

本方案已经落地到 Desktop Shell、Shell Bridge 和偏好设置；Windows 登录启动、静默启动和托盘恢复仍需在目标
机器上执行实机 smoke，macOS/Linux 需要分别验证系统启动项和托盘能力。

## 现状与目标

当前 Desktop 已具备：

- Tauri v2 主窗口和前端首帧后显示机制；
- 系统托盘、窗口隐藏/恢复和明确退出；
- `desktop-preferences.json` 本机偏好文件；
- `getDesktopPreferences` / `setDesktopPreferences` Shell Bridge 扩展；
- `tauri-plugin-single-instance` 单实例处理。

当前缺少：

- Tauri 自启动插件注册；
- 偏好设置中的开机自启和静默启动选项；
- 对自启动进程参数的识别；
- 自启动注册失败后的 Notice、回滚和状态复核。

目标行为：

```text
用户打开“开机自启”
  -> Desktop Shell 注册当前用户的系统启动项
  -> 系统登录后启动 WineStock，并附带固定自启动标记
  -> Shell 读取“静默启动”偏好
  -> 静默启动：保持窗口隐藏，托盘可恢复
  -> 普通启动：前端首帧就绪后显示窗口
```

## 偏好语义

### 开机自启

默认关闭。开启后，Shell 调用 `tauri-plugin-autostart` 的 `enable()`；关闭后调用 `disable()`。

该选项表示当前用户的系统启动注册状态。界面加载时不能只相信 JSON 文件，必须调用插件的
`is_enabled()` 读取系统实际状态，避免用户在系统设置、卸载残留或外部清理后看到过时状态。

注册失败时：

- 不显示为已开启；
- 恢复控件切换前的值；
- 通过统一 Notice 告知用户“无法设置开机自启”，详细原因放在 Notice detail；
- 不影响当前 WineStock 进程、本地服务、托盘和其它偏好。

### 静默启动

默认开启。用户开启开机自启后，下一次系统登录默认保持主窗口隐藏；仅当当前进程由自启动项拉起时隐藏主窗口，
用户手动点击快捷方式启动时仍正常打开窗口。

静默启动依赖托盘恢复入口，因此：

- 自启动关闭时，静默启动控件仍可显示，但应禁用或明确其只对开机自启生效；
- 关闭开机自启时不强制清除静默启动值，重新开启后保留用户选择；
- 托盘初始化失败时不能把窗口永久隐藏，本次启动应按可恢复策略显示窗口；
- 静默启动不停止本地服务，不改变运行模式，不清理登录状态，也不改变 server-mode 防火墙规则。

### 关闭窗口偏好之间的关系

“关闭窗口时”现有选项继续独立生效：

- “最小化到系统托盘”控制用户点击窗口关闭按钮后的行为；
- “退出应用”控制用户点击窗口关闭按钮后是否进入退出流程；
- “静默启动”只控制自启动进程完成前端首帧后的初始可见性。

例如，用户可以选择“开机自启 + 静默启动 + 关闭窗口时最小化到系统托盘”，此时登录后应用在托盘运行，
手动打开窗口后再次点击关闭仍回到托盘。

## Tauri 插件接入

### 依赖与注册

在 workspace 和 `desktop/Cargo.toml` 增加 `tauri-plugin-autostart`，版本沿用 Tauri v2 插件主版本。
在 Tauri Builder 中注册：

```rust
.plugin(tauri_plugin_autostart::init(
tauri_plugin_autostart::MacosLauncher::LaunchAgent,
Some(vec!["--winestock-autostart"]),
))
```

`MacosLauncher::LaunchAgent` 是插件 API 的平台无关初始化参数；Windows、macOS、Linux 由插件选择各自的
系统启动机制。WineStock 不自行写 Windows 注册表、macOS plist 或 Linux desktop entry，也不引入
PowerShell、`windows-sys` 或平台命令行解析。

插件文档中的 `enable()`、`disable()`、`is_enabled()` 可以从 Rust `ManagerExt::autolaunch()` 调用。
由于 WineStock 的前端必须通过 Shell Bridge 使用平台能力，本项目不直接把
`@tauri-apps/plugin-autostart` 暴露给共享前端，也不在 `capabilities/main.json` 增加宽泛的插件权限。

### 启动标记

插件注册时固定附加 `--winestock-autostart`。该参数只用于区分启动来源，不直接表达是否静默：

- `--winestock-autostart` 存在：这是由系统自启动项拉起的进程；
- 参数不存在：这是用户手动启动、单实例恢复或其它普通启动；
- 是否隐藏窗口由参数和 `autostartSilent` 偏好共同决定。

这样切换静默偏好不需要反复改写系统启动项。用户只修改静默设置时，保存偏好即可，下一次系统登录由 Shell
读取新值。

参数识别必须在创建 WebView 前完成，避免窗口短暂显示后又被隐藏：

```text
启动进程
  -> 读取命令行是否包含 --winestock-autostart
  -> 读取 desktop-preferences.json
  -> startupSilent = isAutostartLaunch && autostartSilent
  -> 创建隐藏主窗口和托盘
  -> 前端首帧就绪
  -> startupSilent 且托盘可用：保持隐藏
  -> 其它情况：显示窗口
```

命令行参数不能传递给前端，也不能作为 Shell Bridge 参数返回。它只是 Desktop Shell 的内部启动上下文。

## 本机偏好与 Shell Bridge

### 持久化模型

继续使用 `app_data_dir/desktop-preferences.json`。版本仍为 `1`，新增字段必须使用 serde 默认值兼容已有文件：

```json
{
  "version": 1,
  "closeBehavior": "minimize-to-tray",
  "autostartEnabled": false,
  "autostartSilent": true
}
```

推荐的字段含义：

| 字段               | 所有权        | 读取来源                                        | 默认值             |
| ------------------ | ------------- | ----------------------------------------------- | ------------------ |
| `closeBehavior`    | Desktop Shell | JSON                                            | `minimize-to-tray` |
| `autostartEnabled` | Desktop Shell | 插件 `is_enabled()` 为权威，JSON 只作持久化辅助 | `false`            |
| `autostartSilent`  | Desktop Shell | JSON                                            | `true`             |

旧文件缺少新增字段时，读取为 `true`，不能因为迁移失败重置已有关闭行为；已经明确保存为 `false` 的值继续保留。

### Shell Bridge 契约

扩展现有 Desktop 偏好结构：

```ts
type DesktopCloseBehavior = "minimize-to-tray" | "exit-application";

interface DesktopPreferences {
  version: 1;
  closeBehavior: DesktopCloseBehavior;
  autostartEnabled: boolean;
  autostartSilent: boolean;
}
```

`shell_get_desktop_preferences`：

1. 读取 JSON 中的关闭行为和静默偏好；
2. 调用 `autolaunch().is_enabled()`；
3. 用系统实际状态覆盖 `autostartEnabled` 后返回前端。

`shell_set_desktop_preferences`：

1. 校验版本和布尔字段；
2. 对比系统实际自启动状态；
3. 仅在状态变化时调用 `enable()` 或 `disable()`；
4. 自启动操作成功后持久化全部偏好；
5. 再次读取或确认系统状态后返回权威偏好。

自启动注册成功但 JSON 写入失败时，应尽力回滚系统注册状态，并返回稳定的 Desktop 偏好错误。JSON 写入成功
但插件操作失败时不能返回成功，避免前端显示“已开启”而下次登录实际不启动。

前端的 Notice 只展示失败原因，不增加表单内常驻错误文本，避免改变偏好设置 Dialog 高度和布局。

## 前端偏好设置

在现有“偏好设置”Dialog 增加“启动”分节，Desktop 平台显示，Web/Android 隐藏：

| 选项     | 主文案     | 说明                                               |
| -------- | ---------- | -------------------------------------------------- |
| 开机自启 | `开机自启` | `系统登录后自动启动 WineStock。`                   |
| 静默启动 | `静默启动` | `随系统启动时保持窗口隐藏，可从系统托盘重新打开。` |

实现要求：

- 使用项目已有的二元开关/同意开关视觉模式，不新增原生 Tauri 设置窗口；
- 读取偏好期间两个控件禁用；
- 保存任一项期间两个控件禁用，避免并发写入覆盖；
- 静默启动在开机自启关闭时禁用，但不强制重置其保存值；
- 自启动注册失败、状态读取失败和偏好保存失败全部使用 Notice；
- 成功保存后立即更新本地控件状态，不需要重启当前进程；
- 不因开启自启动而自动关闭当前窗口，也不因开启静默而立即隐藏当前窗口。

“静默启动”只影响下一次由系统启动 WineStock。当前已经运行的实例仍由用户当前窗口操作和托盘规则控制，避免用户保存设置后突然失去窗口。

## 生命周期实现

### 主入口

在 `desktop/src/main.rs`：

- 在 Tauri Builder 注册 autostart plugin；
- 在读取 `DesktopPreferencesState` 后识别启动标记和 `autostartSilent`；
- 将 `startupSilent` 写入 `AppLifecycleState` 或独立的只读启动上下文；
- 主窗口继续使用 `visible(false)` 创建；
- 托盘初始化成功后，前端 `frontendReady` 才根据启动上下文决定显示与否。

### 首帧就绪与兜底

`shell_frontend_ready` 的规则：

- 普通启动：显示、取消最小化并聚焦主窗口；
- 自启动静默且托盘可用：只记录前端已就绪，不显示窗口；
- 自启动静默但托盘不可用：显示窗口，保证用户有可恢复入口。

现有 8 秒前端就绪兜底仍然保留。若静默启动时前端始终没有发送 `frontendReady`，兜底应显示窗口，避免
WebView 初始化失败造成不可见、不可恢复的进程。该例外不代表正常静默启动会显示窗口。

### 托盘和单实例

- 托盘“打开 WineStock”和左键点击始终显示主窗口，覆盖静默启动状态；
- 用户从快捷方式再次启动时，`tauri-plugin-single-instance` 只恢复首个实例，不创建第二个窗口；
- 现有单实例回调不能因为首个实例是静默启动而继续隐藏，用户明确再次启动应得到可见窗口；
- 托盘退出继续走现有 `ExitRequested` 优雅停止流程。

### 服务生命周期

自启动和静默启动都不改变 `DesktopRuntimeManager` 的既有策略：

- 已初始化的 `self-hosted` 或 `server-mode` 仍按现有配置恢复本地服务；
- `server-mode` 静默运行时继续允许其它设备连接；
- 不因窗口隐藏删除防火墙规则；
- `client-only` / `connect-to-remote` 不启动本地服务；
- 明确退出仍停止本地服务并释放资源。

## 失败与兼容策略

### 插件能力失败

插件注册失败属于 Desktop 启动级错误，应记录诊断信息并使偏好中的开机自启不可用或显示关闭；不能让前端误认为
自启动已生效。当前进程仍应尽量加载主窗口和现有运行配置，不能因为用户未开启自启动就阻止基本使用。

读取 `is_enabled()` 失败时，偏好 Dialog 通过 Notice 告知状态读取失败，控件恢复到加载前状态；不能猜测为开启或关闭。

### 托盘不可用

静默启动依赖托盘。若托盘初始化失败：

- 当前启动不保持隐藏；
- 前端首帧或兜底计时器显示主窗口；
- 关闭窗口仍遵循托盘可用性 fail-safe，不把进程隐藏到无法恢复的状态；
- 记录托盘初始化错误，供后续诊断。

### 系统卸载和外部修改

每次打开偏好 Dialog 都以插件 `is_enabled()` 为准。系统启动项被删除后显示为关闭；用户再次开启时重新注册。
JSON 中的旧状态只用于兼容和辅助恢复，不能覆盖系统实际状态。

## 平台边界

| 平台         | 自启动策略                                                        | 静默窗口策略          | 本次范围                   |
| ------------ | ----------------------------------------------------------------- | --------------------- | -------------------------- |
| Windows      | 官方 Tauri autostart provider；优先验证当前用户登录启动           | 托盘隐藏，托盘恢复    | 首要验收平台               |
| macOS        | 官方 Tauri autostart provider；验证 LaunchAgent、签名和登录项行为 | 菜单栏/托盘可用时隐藏 | 先完成编译和能力验证       |
| Linux        | 官方 Tauri autostart provider；验证 XDG 桌面环境                  | 依赖桌面环境托盘支持  | 先完成编译和常见桌面 smoke |
| Android      | 不提供该 Desktop 偏好                                             | 不适用                | 不实现                     |
| Web          | 不提供原生自启动                                                  | 不适用                | 不实现                     |
| Server Shell | 无 WebView 和桌面登录会话                                         | 不适用                | 不实现                     |

“跨平台支持插件”不等于所有桌面环境行为完全一致。macOS 的登录项签名/权限、Linux 的桌面环境托盘能力和
发行包启动项路径必须单独验收，不能在 Windows 验证通过后宣称全部平台可用。

## 实施步骤

1. 增加 `tauri-plugin-autostart` Rust 依赖并在 Desktop Builder 注册固定启动标记。
2. 扩展 Desktop 偏好模型，使用默认值兼容已有 `desktop-preferences.json`。
3. 在 Desktop command 中封装 `is_enabled`、`enable`、`disable`，实现失败回滚和稳定错误。
4. 更新 Shell Bridge TypeScript 契约、Tauri transport、契约校验和相关测试。
5. 在偏好设置 Dialog 增加“启动”分节，接入 Notice 和禁用状态。
6. 在启动入口和 `frontendReady` 中接入自启动标记、静默显示和托盘 fail-safe。
7. 更新 Shell Bridge、平台职责和 Desktop 文档；不要把自启动状态加入共享运行快照或业务 API。
8. 先完成 Windows 实机验证，再进行 macOS/Linux 构建和托盘能力 smoke。

## 验收矩阵

### 自动化检查

- `cargo fmt --check`；
- `cargo test -p winestock-desktop`，覆盖旧偏好迁移、默认值、启动状态和错误回滚辅助逻辑；
- `cargo check -p winestock-desktop`；
- `cd frontend && pnpm build`；
- `cd frontend && pnpm test:shell-bridge-contract`；
- `git diff --check`。

截至 2026-08-02，上述自动化检查均已通过。通过 MCP 对运行中的 Desktop 前端完成了 1280×800
桌面视口和 390×844 移动视口验收：偏好设置 Dialog 的启动分节、禁用状态、内部布局和关闭操作均正常，
没有横向溢出，控制台没有 error/warn。该检查没有切换真实的系统开机自启状态。

### Windows 实机

1. 默认安装后开机自启为关闭，静默启动为关闭。
2. 开启开机自启后，重新登录 Windows，WineStock 自动启动并显示窗口。
3. 开启静默启动后重新登录 Windows，进程存在、托盘存在、主窗口不显示。
4. 静默启动后点击托盘图标，窗口显示、获得焦点且不创建第二个进程。
5. 静默启动后通过快捷方式再次启动，首个实例窗口显示。
6. 关闭开机自启后重新登录，WineStock 不再由系统自动启动。
7. 自启动注册被系统或外部工具删除后，偏好打开时显示实际关闭状态。
8. 模拟插件注册失败，控件恢复原值并显示 Notice，当前应用仍可继续使用。
9. 模拟托盘初始化失败，静默配置不会导致窗口永久不可见。
10. server-mode + 静默启动时，本地服务和 Windows 防火墙规则继续有效，其他设备可以连接。
11. 退出应用后本地服务仍按现有流程停止，不遗留重复进程或端口占用。

### macOS/Linux

- 检查登录项是否按插件规则创建和删除；
- 检查静默启动时菜单栏/托盘入口能否恢复窗口；
- 检查无托盘支持的 Linux 桌面环境是否执行显示窗口的 fail-safe；
- 检查签名、安装位置和更新后自启动路径是否仍有效。

## 不在本次范围

- 不增加系统服务、守护进程、计划任务或自定义注册表实现；
- 不让开机自启绕过单实例保护；
- 不在启动时自动配置 Windows 防火墙或重复请求 UAC；
- 不改变 server-mode、登录、数据库、认证和本地服务恢复语义；
- 不把静默启动实现为退出登录、停止服务或清理前端会话；
- 不为 Android、Web 或无头 Server Shell 增加同名 Desktop 偏好。

## 参考资料

- Tauri v2 官方自启动插件文档：<https://v2.tauri.app/zh-cn/plugin/autostart/>；
- Tauri v2 官方文档中的 Rust API：`tauri_plugin_autostart::init`、`ManagerExt::autolaunch`、`enable`、
  `disable`、`is_enabled`；
- KeyWine 原理参考：`D:\Project\kb\KeyWine\src-tauri\src\main.rs`、
  `D:\Project\kb\KeyWine\src-tauri\src\app\settings.rs`、
  `D:\Project\kb\KeyWine\src\views\AppSettings.vue`。
