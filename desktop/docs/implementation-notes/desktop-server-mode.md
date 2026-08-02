# Desktop LAN Server Mode 实现方案

## 结论

整改前 Desktop 已经具备启动本地 `core` 的能力，但只实现了 `self-hosted`：服务只允许绑定 loopback，前端通过
`http://127.0.0.1:<port>` 使用它。目标中的 `server-mode` 不是无头 server，而是“Desktop UI + 本地 Axum 服务 +
其它设备 HTTP 客户端”的局域网主机模式。该模式在协议、共享配置模型和前端页面中已经预留；整改前 Desktop 壳在
配置校验阶段明确拒绝它，并把 `capabilities.serverMode` 固定为 `false`。

因此本功能不需要新增服务实现，也不应把 Desktop 改造成 `server/` 的无头 shell。实现方式是扩展现有
`DesktopRuntimeManager`，使它在 `server-mode` 下启动同一个 `winestock_core::start_local_service()`，绑定可被
其它设备访问的地址；Desktop 自身继续加载并显示打包的共享前端，发布真实的局域网访问 URL，并继续由共享前端
呈现配置、状态和错误。

## 当前现状

| 部分           | 整改前状态                                                                                    | 当前实现                                    |
|--------------|-----------------------------------------------------------------------------------------|------------------------------------------------------|
| `shared`     | 已有 `RuntimeMode::ServerMode`；服务模式被视为本地服务；server-mode 端口要求 `1..65535`                    | 不需要改变配置枚举和基本端口规则                                     |
| `core`       | `start_local_service()` 统一完成 bind、bootstrap、serve、shutdown；绑定地址由配置决定                    | 直接复用，不增加 Desktop 专用 API                              |
| `server`     | 独立的无头 shell 已能读取配置并启动本地服务                                                               | 作为另一种部署入口，不定义 Desktop `server-mode`，不被 Desktop 调用或复制 |
| Desktop 配置校验 | `prepare_config()` 拒绝 `server-mode`；`self-hosted` 只允许 loopback                          | 已接受有效 IP；server-mode 固定端口，self-hosted 规则保持不变                |
| Desktop 能力   | `desktop_capabilities()` 固定 `serverMode: false`                                         | Desktop `serverMode: true`，本地生命周期仍按归属动态开放            |
| Desktop 快照   | Rust `RuntimeServiceSnapshot` 没有实际填充 `lanAccessUrls`                                    | 已发布真实 LAN 地址，并在状态切换时清理/刷新        |
| 前端协议         | `RuntimeSnapshot.service.lanAccessUrls`、`getUsableLanAccessUrls()`、地址 Dialog 和设置页入口已经存在 | 主要是接收真实数据、补测试，不重建 UI                                 |
| Android      | 当前明确禁用 server-mode                                                                      | 本方案不改变 Android 策略                                    |

## 当前实现

本方案已在 Desktop 壳完成首版实现：

- `prepare_config()` 接受 `server-mode`，允许有效 IP 监听；`self-hosted` 仍只允许 loopback。
- Desktop 复用 `winestock_core::start_local_service()`；WebView 对 wildcard 监听使用 loopback，具体监听地址使用实际绑定 IP。
- server-mode 使用固定端口，端口占用返回 `port_in_use`；只有 self-hosted 保留动态端口重试。
- Rust Shell Bridge 快照已发布 `lanAccessUrls`；当前 Windows 通过 IP Helper 读取运行中的 IPv4 网卡，只返回 RFC1918 私网地址，
  非 Windows 暂为空列表；后续迁移到 `if-addrs` 后统一覆盖 Windows/macOS/Linux。
- 停止、启动中、失败、崩溃和切换远端时不保留旧 LAN 地址；没有合格网卡时服务仍保持 running、地址列表为空。
- `capabilities.serverMode` 已对 Desktop 开放；前端既有运行设置页和地址 Dialog 直接消费真实快照，不新增第二套 UI。

首版明确不发布 IPv6、VPN/公网 IPv4，也不自动写入 Windows 防火墙规则；LAN 地址发现当前仍是 Windows-only，
后续应按跨平台方案迁移；防火墙集成的独立方案见
[`desktop-firewall-access.md`](desktop-firewall-access.md)。这不改变 Android `server-mode` 策略。

## 已执行验证

已完成：

- `cargo test -p winestock-desktop`：Desktop 单元与 runtime manager 集成测试通过，覆盖 server-mode 启动、冷启动、固定端口冲突、IP/端口校验、焦点刷新和切换远端清理地址。
- `cargo check -p winestock-desktop` 与 `cargo fmt --all -- --check`。
- `frontend` 的 `test:lan-access`、`test:shell-bridge-contract` 和生产构建。
- `pnpm desktop:build`：Windows x64 Tauri release 与 NSIS 安装包构建成功，产物为
  `target/release/bundle/nsis/WineStock_0.1.0_x64-setup.exe`。
- `git diff --check`。

仍需安装包安装和至少一台同网设备的跨设备 HTTP smoke；当前测试没有把防火墙、路由器隔离或外部设备可达性伪装成已验证。

## 范围与非目标

本次实现包含：

- Desktop 配置中的 `server-mode` 校验、应用、冷启动恢复和端口冲突处理。
- Desktop 本地服务的正常停止、重启、异常退出和 Tauri 进程退出清理。
- 真实监听地址与真实局域网访问 URL 的计算、去重和状态发布。
- Shell Bridge v1 的 Rust/TypeScript 契约同步，以及前端运行设置页的能力接入。
- Windows 防火墙、网络切换、端口冲突、跨设备 HTTP smoke 验证文档和测试。

本次不包含：

- 新增 Axum 路由、业务 API、数据库表或认证协议。
- 把 Desktop 改造成无头 `server` 进程；Desktop `server-mode` 仍保留 Tauri 窗口和共享前端，无 UI 部署另使用 `server/`。
- 让 Axum 服务前端静态资源，WebView 仍加载 Tauri 打包的 `frontend/dist`。
- Android `server-mode`、Foreground Service、系统通知或后台常驻。
- 自动修改 Windows 防火墙规则。首版只检测并提示防火墙可能阻止访问，是否自动放行应另立安全设计。
- TLS 证书、反向代理、互联网暴露或公网部署。Desktop server-mode 首版只定义 HTTP 局域网访问。

Windows 防火墙不是本首版 server-mode 实现的一部分。当前实现只发布 LAN URL 并在无地址时提示用户检查
防火墙；规则写入、UAC 和 Public/组策略处理按 [`desktop-firewall-access.md`](desktop-firewall-access.md)
单独实施。

## 目标架构

```text
Tauri main process
  -> DesktopRuntimeManager
       -> shared AppConfig(mode=server-mode, bind_host, port)
       -> winestock_core::start_local_service()
       -> RunningLocalService
       -> LAN address provider
       -> Shell Bridge RuntimeSnapshot/event

frontend WebView
  -> packaged frontend/dist
  -> HTTP 本机 API 地址                         # wildcard 用 loopback，具体绑定地址用实际 IP

其它设备
  -> HTTP http://<real-lan-ip>:<actual-port>   # 连接 Desktop 提供的同一 core 服务
```

关键边界：

- `core` 只负责服务和业务，不知道自己由 Tauri `server-mode`、Tauri `self-hosted` 还是无头 `server` shell 启动。
- `DesktopRuntimeManager` 负责配置和进程级服务句柄，但不代理业务 HTTP。
- Desktop 本机前端通过 Shell 返回的本机 API 地址使用服务；其它设备直接通过局域网 HTTP 使用同一个服务，不加载 Desktop 的
  WebView 页面，也不经过 Shell Bridge。
- Desktop 前端只消费 Shell Bridge 的 `apiBaseUrl`、`boundAddress` 和 `lanAccessUrls`，不枚举网卡，也不从
  `bindHost` 拼接访问地址。
- Tauri command 只暴露已经存在的具名 Shell Bridge 方法；不新增任意 native command。

Tauri v2 的实现依据是：应用状态通过 `Builder::manage` 注册、command 通过 `State` 获取；应用退出可在
`RunEvent::ExitRequested` 中阻止默认退出并完成异步清理；具名 command 继续通过 capability/permission 绑定到
主窗口。对应官方文档见：

- [Calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
- [Capabilities](https://v2.tauri.app/security/capabilities/)
- [Permissions](https://v2.tauri.app/security/permissions/)

## 配置和网络规则

### 配置映射

`server-mode` 使用现有字段，不增加 `server.enabled`：

```json
{
  "mode": "server-mode",
  "bindHost": "0.0.0.0",
  "port": 17890,
  "remoteBaseUrl": ""
}
```

规则如下：

- `mode=server-mode` 时 `port` 必须是 `1..65535`，不能使用 `0` 动态端口。
- `bindHost` 必须是有效 IP 地址；允许 `0.0.0.0`、`::`、具体 IPv4 和具体 IPv6。
- `remoteBaseUrl` 在本地模式下忽略其连接语义并规范化为空或保留无害草稿，不作为服务地址。
- `auto_start_server` 由 Desktop 内部固定为 `true`，不作为前端设置项。
- server-mode 配置只有在 bind、core bootstrap 和服务启动成功后才持久化；失败时保留之前可用配置。
- `self-hosted` 的既有规则保持不变：只允许 loopback，可在首次 apply 或冲突重试时使用临时端口 `0`。

### 访问地址分离

必须继续区分三个地址：

- `bindHost`：Axum 实际监听地址，例如 `0.0.0.0`。
- `service.boundAddress`：实际绑定的 socket，例如 `0.0.0.0:17890`，只用于状态诊断。
- `service.apiBaseUrl`：Desktop WebView 自己使用的地址；wildcard 监听时使用 loopback，具体监听地址时使用
  实际绑定 IP，不能返回 `0.0.0.0` 或 `::`。
- `service.lanAccessUrls`：Shell 根据真实网卡地址返回的其它设备可用地址，例如
  `http://192.168.1.23:17890`。

当绑定具体 IPv4 地址而非 wildcard 时，只发布该实际绑定地址；wildcard 才从真实网卡状态计算全部可用地址。
不能简单把 `bindHost` 当成可分享地址，因为 `127.0.0.1`、IPv6 loopback 和 wildcard 都不能被其它设备使用。

### 局域网地址计算

新增 desktop 内部的地址提供模块，已放在 `desktop/src/lan_access.rs`，职责仅限于读取当前机器的
网络接口并格式化 URL：

1. 在服务已经成功 bind、拿到实际端口后读取接口地址。
2. 迁移目标使用跨平台高层 `if-addrs` 接口；不调用外部 `ipconfig.exe`、`ifconfig` 或其它命令，也不依赖
   命令输出语言或用户 PATH。Windows Firewall 的高层 `windows` crate 与地址发现保持职责分离。
3. 过滤 down 接口、未指定地址、loopback、multicast、广播地址和重复地址。
4. 首版优先发布可直接输入的 IPv4 私有地址；如支持 IPv6，只发布可脱离 scope 的 ULA 地址，链路本地地址
   需要处理 zone/scope，未完成前不发布，避免生成用户无法使用的 URL。
5. 具体 IPv4 监听只发布该地址；wildcard 按稳定顺序去重，接口顺序由系统返回，地址顺序在同一接口内保持；不得返回 `0.0.0.0`、`::`、`127.*`、
   `[::1]`、占位文本、路径、查询参数、凭据或业务路径。
6. 对每个地址使用当前真实端口生成根 URL，格式统一为 `http://host:port/` 的 origin 形式；传给前端前
   去掉末尾 `/` 或由前端规范化，但两侧测试必须统一。

网络接口变化时，不要求首版后台持续监听系统网络事件。以下事件会重新计算并发布快照：启动成功、重启成功、
前端请求刷新/重新应用配置、应用恢复焦点。服务停止或失败时清空 `lanAccessUrls`。

如果未来需要无感知地响应 Wi-Fi 切换，再单独增加 Windows 网络变化监听；不能让前端自行轮询网卡。

## Shell Bridge 契约变化

### Rust DTO

在 `desktop/src/contract.rs` 的 `RuntimeServiceSnapshot` 增加：

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub lan_access_urls: Option<Vec<String> >,
```

所有构造函数必须显式处理该字段：

- `server-mode + running`：返回非空时的真实地址列表；没有合格网卡时返回 `None` 或空数组。
- `self-hosted + running`：返回 `None` 或空数组，不向 UI 暗示 LAN 服务可用。
- `remote`、`stopped`、`starting`、`failed`：返回 `None` 或空数组。

`RuntimeCapabilities` 改为按平台能力和当前运行配置返回：

- Desktop `serverMode=true`，表示该平台允许应用 server-mode 配置。
- `startLocalService`、`stopLocalService`、`restartLocalService` 仍只在已初始化且当前归属为 local 时为 true。
- Android 和 Web 的 `serverMode=false` 不变。

如 `lanAccessUrls` 属于已有 TypeScript 可选字段，不需要提升 Shell Bridge 协议版本；仍需更新契约测试，确保
Rust 发布的 payload 能通过 `assertCompatibleRuntimeSnapshot()`。

### Command 与权限

不新增 command。现有以下方法足以覆盖流程：

- `shell_get_runtime_snapshot`
- `shell_validate_runtime_config`
- `shell_apply_runtime_config`
- `shell_start_local_service`
- `shell_stop_local_service`
- `shell_restart_local_service`
- `shell_frontend_ready`
- `shell_open_external`

`capabilities/main.json` 和 `permissions/shell-bridge.toml` 继续采用具名 command 集合。Tauri capability 不应因为
server-mode 而开放通用 shell、文件系统或任意网络 native API。

## Desktop RuntimeManager 改造

### 配置校验

修改 `desktop/src/runtime.rs::prepare_config()`：

- 删除 `server-mode` 的 “尚未支持” 错误。
- `self-hosted` 继续要求 loopback。
- `server-mode` 接受有效 IP；明确拒绝空字符串、域名、非法 IPv4/IPv6。
- server-mode 固定端口校验沿用 shared 规则。
- `shared_config()` 已经能映射 `RuntimeMode::ServerMode`，保持该映射并让 storage 路径继续由 Desktop 派生。

### 启动和恢复

现有 `initialize()`、`start_or_apply()`、`start_service()`、`restore_from_previous()` 和 monitor 可以继续作为
唯一生命周期路径，但必须取消只针对 `SelfHosted` 的隐含判断：

- 已保存且有效的 server-mode 配置冷启动时自动启动本地服务。
- server-mode 首次 apply 使用用户固定端口，不自动改成 `0`。
- server-mode bind 失败且错误为 `AddrInUse` 时返回 `port_in_use`，不静默换端口。
- server-mode 启动成功后保存实际端口、绑定地址和 LAN URL 快照。
- 从 server-mode 切到 remote 时先 graceful shutdown，再保存远端配置。
- 从 remote/self-hosted 切到 server-mode 时，旧本地服务先停止，再以新 bind/port 启动；新服务启动失败时尽力恢复
  旧服务和旧快照。
- 进程退出仍由 `RunEvent::ExitRequested` 阻止默认退出，调用同一个 `shutdown_local_service()`，完成后显式退出。

### 快照状态

建议把 `LocalServiceDetails` 扩展为：

```text
bound_address
api_base_url
local_auth_exchange_token
lan_access_urls
```

`map_start_result()` 在拿到 `RunningLocalService::info()` 后计算这些值。计算失败不应让已经成功启动的 HTTP 服务
变成 failed；应记录受控状态错误或返回空地址列表，并保留本机 `apiBaseUrl`，因为局域网地址展示是附加能力，不是
core 服务启动条件。

### 异常退出

现有 monitor 发现 `RunningLocalService` task 结束后发布 `failed`。server-mode 下同样执行，并清空 LAN 地址；
不自动无限重启，避免端口冲突和数据库错误形成重启循环。前端通过已有运行设置页提供重试。

## 前端改造

前端运行设置和地址 Dialog 已有 server-mode 的主要 UI，现有“允许其他设备连接”文案和交互方向正确，不应新建
第二套设置界面。实施时检查并补齐：

- `snapshot.capabilities.serverMode` 为 true 时 Desktop 显示并允许选择“允许其他设备连接”。
- server-mode 只显示固定端口；监听地址放在高级设置，默认建议 `0.0.0.0`，但明确说明这是监听语义，不是分享地址。
- 保存前继续使用已有的局域网开放确认和本机管理员密码门，防止把随机占位密码直接开放给其它设备。
- 保存成功后只从 `snapshot.service.lanAccessUrls` 展示地址；前端不枚举网卡、不根据 `bindHost` 生成地址。
- 地址列表为空时显示“当前设备没有可用的局域网地址”，并提示检查网络适配器和防火墙。
- 服务停止、启动中、失败、切回 remote/self-hosted 时关闭地址 Dialog，并清除旧地址。
- 保持 HTTP 明文风险提示。首版没有 TLS，不得把局域网地址描述成互联网安全地址。

重点文件：

- `frontend/src/shell/contract.ts`
- `frontend/src/shell/lanAccess.ts`
- `frontend/src/pages/RuntimeSettingsPage.vue`
- `frontend/src/pages/runtime-settings/model.ts`
- `frontend/src/components/runtime/LanAccessDialog.vue`

## 测试与验收

### Rust 单元/集成测试

在 `desktop/tests/runtime_manager.rs` 增加：

- `server-mode` 配置校验接受 `0.0.0.0`、具体 IPv4 和合法 IPv6。
- `server-mode` 拒绝 loopback 不是必须条件，但拒绝非法 IP、空 host、域名和端口 `0`。
- Desktop capability 在 `server-mode` 可用时返回 `serverMode=true`。
- server-mode 使用固定端口，端口占用返回 `port_in_use`，不自动动态端口重试。
- server-mode 成功启动后 `apiBaseUrl` 对 wildcard 使用 loopback、对具体监听使用实际 IP；`boundAddress` 是真实监听地址，LAN URL 不包含 wildcard。
- 切换到 remote 会停止本地服务；应用退出释放端口。
- 配置保存失败或新服务启动失败时，旧运行服务和旧配置尽力恢复。
- 服务异常退出发布 failed 快照且不保留陈旧 LAN 地址。

LAN 地址提供模块单独测试纯函数：

- 过滤 loopback、wildcard、multicast、重复地址。
- IPv4/IPv6 URL 格式正确。
- 没有可用接口时返回空结果。
- 端口始终使用实际绑定端口。

### 前端测试

扩展现有 `frontend/tests/lanAccess.test.mjs` 和 `shellBridgeContract.test.mjs`：

- 接受 Desktop server-mode 的真实快照。
- 保持拒绝 wildcard、loopback、凭据、路径、查询参数和占位地址。
- server-mode 未 running、无 capability 或 ownership 为 remote 时不展示地址。
- `lanAccessUrls` 地址变更时 Dialog 和旧地址状态正确清理。

### Windows smoke 验收

最小验收矩阵：

| 场景                             | 预期                                              |
|--------------------------------|-------------------------------------------------|
| 首次启动未配置                        | 前端正常加载；不启动 core；server-mode 可选但未生效              |
| 选择 server-mode，`0.0.0.0:17890` | 弹出确认；成功后本机仍用 `127.0.0.1:17890`，显示真实 LAN URL     |
| 同机浏览器访问                        | `GET /api/health` 成功；WebView 业务请求正常             |
| 同局域网另一设备访问                     | 通过展示的真实 URL 访问 `/api/health` 成功；不要求输入 `0.0.0.0` |
| 无网络适配器/仅 loopback              | 服务可运行；LAN 地址为空；UI 提示无可用局域网地址                    |
| 固定端口已占用                        | 保存失败，显示 `port_in_use`，不改成随机端口                   |
| Windows 防火墙阻断                  | Shell 状态仍为 running；UI/验收记录提示网络不可达可能来自防火墙        |
| 切回 self-hosted                 | 旧服务停止，新服务只绑定 loopback，LAN 地址清空                  |
| 切回 remote                      | 本地端口释放，WebView 使用远端 URL                         |
| 关闭应用                           | Tauri 等待 graceful shutdown，端口释放，可立即重启           |
| Wi-Fi 地址变化后恢复焦点                | 重新计算并发布地址，旧地址不再展示                               |

按项目完成门槛，涉及 UI 时还要覆盖 `1440x900`、接近 `768px` 的窄桌面和 `390x844`，检查真实尺寸、溢出、
异步状态和浏览器控制台；涉及跨设备访问时必须在 Windows 主机和至少一台同网设备上执行 HTTP smoke。

## 实施顺序

1. 先补 Rust contract 的 `lanAccessUrls` 字段、server-mode capability 和对应快照构造函数。
2. 修改 `prepare_config()` 与 `start_service()` 的模式分支，确保 server-mode 固定端口和 bind 规则正确。
3. 新增并测试 `lan_access.rs`，在服务成功 bind 后生成地址。
4. 将地址写入 `LocalServiceDetails`、running/failed/stopped 快照和状态事件。
5. 复核前端设置页、桥契约校验和地址 Dialog，补齐 server-mode Desktop capability 流程。
6. 补 Rust/Node 测试，再执行 Desktop build、前端 build 和 Windows 跨设备 smoke。
7. 最后更新 desktop 代码地图、组件文档中的当前状态和安装/验收记录；不要在实现完成前把架构文档写成“已支持”。

## 风险与决策点

- **网络接口 API**：优先使用 Windows API 和现有 `windows` crate，避免引入依赖或解析 `ipconfig` 文本；若实现
  需要新 crate，按 `docs/agent-checklist.md` 先核对版本、所有权和锁文件影响。
- **IPv6**：首版可以只发布可直接使用的 IPv4 私有地址，明确记录 IPv6 暂不展示；不要发布未经 scope 处理的
  `fe80::` 地址。
- **防火墙**：服务监听成功不等于局域网可达。首版不自动改系统安全策略，验收必须区分“服务未启动”和“防火墙/网络
  不可达”。
- **认证**：server-mode 是共享数据入口，已有前端密码门必须保持；Shell Bridge 不新增 token 传递，也不把本机
  静默换取凭据暴露给远端设备。
- **网络切换**：先用恢复焦点和显式重启触发地址刷新；持续网络事件监听属于后续增强，不应阻塞首版 server-mode。
