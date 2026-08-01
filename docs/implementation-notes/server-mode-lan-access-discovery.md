# Server mode 本机局域网地址实施方案

> 文档状态：阶段 A 已实施；API 33 Android 远端 smoke 已完成，正式平台地址提供待后续<br>
> 涉及组件：`frontend`、Desktop/Android Shell、根项目文档<br>
> 编制日期：2026-07-23<br>
> 当前边界：前端展示与契约消费已完成；Android 当前 `serverMode = false` 的隐藏行为已上机验证，
> 正式 Desktop Shell 地址发现和 Android server mode 的真实 LAN 地址仍待实现

> 实施记录：纯地址选择器、自动化测试、本机运行设置主入口、头像快捷入口、共用 Dialog、复制反馈、
> 地址失效关闭和 Web fallback 占位值清理均已完成。正式 Shell 仍需按本文提供真实 `lanAccessUrls`。

## 1. 结论

采用“本机运行设置主入口 + 头像菜单快捷入口 + 通用 Dialog”的方案：

- `/settings/runtime` 的“当前服务”区域提供“本机局域网地址”主入口，便于配置和排障时发现。
- 已登录应用壳的头像 Popover 在满足展示条件时增加“本机局域网地址”快捷入口。
- 两个入口打开同一个 `LanAccessDialog`，不复制地址列表、复制反馈或安全提示。
- 地址只读取 `RuntimeSnapshot.service.lanAccessUrls`；前端不枚举网卡、不根据 `bindHost` 拼接地址，也不新增业务 HTTP API。
- Web fallback 无法可靠发现宿主机网卡，因此不得返回或展示 `http://<局域网地址>:port` 一类占位值。

头像菜单适合作为高频快捷入口，但不应成为唯一入口：未登录、服务异常或用户正在配置运行模式时，本机运行设置页仍是稳定可达的 Shell 控制界面。

## 2. 目标与非目标

### 2.1 目标

- 让 server mode 的操作者快速查看其它设备实际可以输入的全部连接 URL。
- 明确区分本机自用的 `apiBaseUrl`、监听用的 `boundAddress` 和对外分享的 `lanAccessUrls`。
- 在服务状态或网卡变化后使用最新 Shell 快照更新地址，不保留失效副本。
- 提供逐项复制、键盘焦点、Escape、遮罩关闭和 Android 原生返回协同。
- 不展示 wildcard、loopback、占位符或结构无效的地址。

### 2.2 非目标

- 不让前端通过 WebRTC、浏览器实验 API 或 User-Agent 猜测网卡地址。
- 不让 core/Axum 为平台网络信息增加业务 HTTP 接口。
- 不在第一版引入二维码依赖、网卡名称、网络测速或在线设备扫描。
- 不把 `0.0.0.0`、`::` 当作浏览器可访问 URL。
- 不在 Android 的后台服务、通知和系统限制尚未完成时提前开启 `serverMode` capability。

## 3. 数据流与所有权

```text
Desktop / Android Shell
  -> 枚举当前可用网络接口
  -> 结合实际监听端口生成可连接 HTTP URL
  -> 写入 RuntimeSnapshot.service.lanAccessUrls
  -> 网络或服务状态变化时发布新快照

frontend shell/lanAccess.ts
  -> 校验运行模式、ownership、phase 与 capability
  -> 规范化、过滤、去重，保留 Shell 给出的优先顺序

AppShell / RuntimeSettingsPage
  -> 决定入口是否可见与 Dialog 开关

LanAccessDialog
  -> 只负责地址呈现、复制反馈和安全提示
```

职责边界：

- Shell 拥有网卡发现、地址有效性来源、服务端口和状态更新。
- `frontend/src/shell/lanAccess.ts` 拥有跨页面一致的防御性清理与显示条件。
- `AccountPopover` 保持纯展示，只接收 prop 并发出 typed event。
- `AppShell` 持有全局快捷 Dialog 状态；`RuntimeSettingsPage` 持有页面入口状态。
- `LanAccessDialog` 复用 `ModalDialog`，不读取运行快照或管理 Shell 生命周期。

## 4. 展示条件

只有同时满足以下条件，两个入口才可见：

```ts
snapshot.config.mode === "server-mode" &&
  snapshot.service.ownership === "local" &&
  snapshot.service.phase === "running" &&
  snapshot.capabilities.serverMode &&
  usableLanAccessUrls.length > 0;
```

这组条件避免以下误展示：

- 当前只是草稿选择了 server mode，但尚未应用；
- 当前连接的是远端服务；
- 本地服务正在启动、停止、失败或已停止；
- 平台尚未实现 server mode；
- Shell 没有返回任何真实可连接地址。

Dialog 打开后若新快照不再满足条件，应关闭 Dialog；不得继续展示旧地址。

## 5. 地址清理规则

`lanAccessUrls` 属于 Shell Bridge 数据，前端仍需在展示前做防御性处理：

1. 去除首尾空白，只接受 `http:` 或 `https:` URL。
2. 拒绝用户名、密码、查询参数、片段和非根路径。
3. 拒绝包含尖括号等占位符标记的值。
4. 拒绝 `0.0.0.0`、`::` 及其等价未指定地址。
5. 拒绝 `localhost`、`*.localhost`、IPv4 `127.0.0.0/8` 和 IPv6 loopback。
6. 使用 URL origin 形成稳定展示值，去掉无意义的尾部 `/`。
7. 按规范化结果去重，并保留 Shell 返回的顺序；接口优先级由 Shell 决定，前端不猜测 Wi-Fi、以太网或 VPN 的优先级。

第一版允许 Shell 返回 IPv4、IPv6 或可解析的局域网主机名，不仅限 RFC1918 IPv4；企业网络、IPv6 ULA 和平台路由策略仍由 Shell 判断。

## 6. UI 与交互

### 6.1 头像 Popover

操作顺序保持为：

1. 本机运行设置；
2. 本机局域网地址（条件满足时）；
3. 退出登录。

点击局域网入口后先关闭 Popover，再打开 Dialog，并把 Dialog 关闭后的焦点返回头像触发按钮。

### 6.2 本机运行设置

“当前服务”区域在真实地址可用时显示“本机局域网地址”按钮。该区域是未登录和故障恢复场景也能访问的主要入口，不把当前设备信息只藏在账户菜单内。

### 6.3 Dialog

- 标题：`本机局域网地址`；说明这些地址属于当前设备，避免被理解为账号或当前远端服务的信息。
- 每个地址独立成行，完整 URL 可换行且可选中，并提供项目统一的线性复制图标按钮；按钮包含
  `title` 和带完整地址的 `aria-label`。
- 复制成功或失败使用全局 Notice，不在地址行内插入会改变布局的临时文字。
- 提示接入设备需处于可达网络，连接失败时检查防火墙；同时提示 HTTP 明文地址只应分享给可信网络内设备。
- 复用 `ModalDialog` 的遮罩、焦点陷阱、Escape、离场动画和 native-back registry。

## 7. 分阶段实施

### 阶段 A：本次已实施

- 新增纯函数地址选择器及 Node 自动化测试。
- 新增共用 `LanAccessDialog`。
- 接入 `AccountPopover`、`AppShell` 和 `RuntimeSettingsPage`。
- 删除 Web fallback 的占位 LAN URL。
- 更新本机运行设置文档、Shell Bridge 规则和前端代码地图。

### 阶段 B：正式 Desktop Shell

- Tauri shell 启动本地服务后枚举当前有效接口。
- 基于实际端口生成 URL，并在接口、网络和服务状态变化时刷新快照。
- 明确防火墙授权或失败提示，不把系统权限处理交给前端。

正式 `desktop` 当前尚未实现，因此阶段 A 只能完成消费端，不能宣称 Desktop 真实地址发现已通过。

### 阶段 C：Android server mode

- 完成嵌入式 core、前台服务、通知、后台限制和网络策略后再开启 capability。
- Android Shell 使用相同快照字段提供真实地址；前端无需建立第二套 UI。
- 当前 Android `serverMode = false`，入口必须保持隐藏。

### Headless server

`server` 没有 WebView 和头像菜单。其控制台可独立完善多地址启动输出，但不得为了复用前端 UI 让 Axum 打包或托管 frontend。

## 8. 验收矩阵

| 场景                                                         | 预期                            |
| ------------------------------------------------------------ | ------------------------------- |
| server mode、本地服务 running、capability 开启、存在真实 URL | 两个入口可见，打开同一地址列表  |
| self-hosted / remote mode                                    | 入口隐藏                        |
| starting / stopping / stopped / failed                       | 入口隐藏；已打开 Dialog 关闭    |
| wildcard、loopback、占位符、非法 URL                         | 对应值不展示                    |
| 多个等价或重复 URL                                           | 只显示一次                      |
| 快照更新为新地址                                             | 列表使用新快照，不保留旧副本    |
| 复制成功 / 失败                                              | 显示全局 Notice，列表尺寸不变化 |
| Web fallback server mode                                     | 不伪造地址，入口隐藏            |
| Android 当前实现                                             | capability 关闭，入口隐藏       |

自动化和静态检查：

- 地址清理 Node test；
- `vue-tsc` 与 Vite production build；
- Prettier targeted check；
- `git diff --check`；
- 桌面 `1440 × 900`、断点附近和移动 `390 × 844` 的 Dialog、溢出、焦点与控制台检查。

本次实际验收结果：

- 地址清理 Node test 4 项通过，原生返回回归 test 8 项通过；
- `pnpm build` 通过；
- `1440 × 900`、断点附近和 `390 × 844` 均无页面或 Dialog 横向溢出；
- 头像 Popover 在桌面与移动端均显示条件化入口，移动端三项操作完整可达；
- 复制图标按钮尺寸、提示和无障碍名称正确，浏览器剪贴板成功路径及全局 Notice 已验证；
- 快照切换为 stopped 并移除地址后，入口立即隐藏，Dialog 在统一离场动画完成后移除；
- 浏览器控制台没有新增 error、warning 或 issue。
- 2026-07-23 在 Xiaomi `M2012K11AC`、Android 13 / API 33 真机连接远端 HTTP 服务后，健康检查、
  登录和 dashboard 均正常；账户菜单只显示“本机运行设置”和“退出登录”，没有“本机局域网地址”，
  符合 Android 当前 `serverMode = false` 与非本地 server-mode 服务不展示入口的约束。

## 9. 剩余覆盖项

- 正式 Desktop Shell 尚不存在，仍需在实现后用真实 Shell 快照验证网卡发现、网络变化和防火墙场景。

- Android server mode 尚未启用；完成 Foreground Service、通知与网络策略后，仍需补充真实 LAN URL、
  复制、旋转、前后台切换和原生返回验收。当前 API 33 设备结果只证明 capability 关闭时入口正确隐藏，
  不代表 Android 已具备对外提供 server-mode 地址的能力。
