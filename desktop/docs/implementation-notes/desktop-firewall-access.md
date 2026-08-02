# Desktop server-mode 防火墙与跨平台 LAN 访问方案

## 结论

当前 Desktop `server-mode` 已经能够让 `winestock_core` 监听局域网地址，也能通过 Shell Bridge 发布
`lanAccessUrls`。Windows 首版已由 Desktop 壳创建并维护 WineStock 自有的 Windows Defender Firewall
入站规则；“服务 running”仍只证明 core 已监听，是否具备局域网访问条件由快照中的 `firewall` 状态单独表示。

推荐由 Desktop 壳补齐一条由 WineStock 自己管理的、只允许局域网访问当前 TCP 端口的 Windows 防火墙规则。
`core`、`shared`、无头 `server` shell 和业务 HTTP 契约不感知 Windows 防火墙；需要管理员权限的动作交给一个
受限的、按需以 UAC `runas` 启动的 helper 完成。主 Tauri 进程不整体提权，也不向前端开放通用命令执行能力。

当前文档同时记录已落地的 Windows 首版实现和 macOS/Linux 后续 provider 方案；后续 provider 不应改变
Shell Bridge 中“服务生命周期”和“防火墙状态”分离的语义。

## LAN 地址发现与多平台策略

局域网地址发现和防火墙放行是两个独立能力，不能因为 Windows 有 IP Helper 就把地址发现设计成
地址发现不再是 Windows-only。当前 `desktop/src/lan_access.rs` 使用 `if-addrs` 0.15 在 Windows、macOS 和 Linux 统一枚举
接口地址，服务监听成功后各平台都可以执行同一套私网 IPv4 过滤和 URL 生成。

推荐把地址发现改成跨平台高层接口，优先采用 `if-addrs` `0.15.x`：

- Windows、macOS、Linux 统一调用同一套安全 Rust API；由库内部适配 Windows IP Helper、Unix
  `getifaddrs` 等系统实现。
- `lan_access.rs` 只处理 `Interface`/地址值，不直接遍历 Windows 链表，不手工释放系统缓冲区，也不依赖
  `ipconfig`、`ifconfig` 或系统命令的语言输出。
- 地址过滤、RFC1918 判断、去重、具体绑定地址限制和 URL 格式化保持平台无关。
- 防火墙实现不放入这个模块；地址发现失败只能产生空地址/诊断状态，不能伪装成防火墙已放行。

本项目保留高层 `windows` crate 用于 Firewall COM 和 UAC，网卡地址则统一交给 `if-addrs`，避免在 Desktop
业务模块中处理 `GetAdaptersAddresses` 的原始链表和指针生命周期。

### 平台能力矩阵

| 平台 | LAN 地址发现 | 防火墙自动管理 | server-mode 首版策略 |
| --- | --- | --- | --- |
| Windows Desktop | `if-addrs` | 高层 `windows` crate + Firewall COM + UAC helper | 已实现：自动放行 Domain/Private 的 LocalSubnet |
| macOS Desktop | `if-addrs` | 可通过特权 helper 管理 PF；Application Firewall 只能做应用级放行 | 当前 capability 关闭，provider 验证后再开放 |
| Linux Desktop | `if-addrs` | 可通过 firewalld D-Bus 或受控 nftables provider 管理 | 当前 capability 关闭，provider 验证后再开放 |
| 无头 `server` | shell 自己输出地址 | 由部署管理员配置 | 不由 WineStock 自动写主机防火墙 |
| Android | Android 原生网络能力 | 不适用当前方案 | 现有 `server-mode` 继续禁用 |
| Web | 浏览器能力 | 不适用 | 不启动本地服务 |

macOS/Linux 技术上都可以自动管理，但不能共用 Windows 的规则实现：

- macOS 的 Application Firewall（`socketfilterfw`）主要按应用放行，不是当前端口规则；直接放行 Desktop
  可导致该应用的其它监听端口也被允许。若要保持“只开放当前端口”，应由签名的特权 helper 管理 PF anchor，
  只加载 WineStock 自有的 TCP 端口规则，并在停止/卸载时卸载或清理该 anchor。
- Linux 可能由 UFW、firewalld、nftables、iptables 或企业安全软件管理。推荐优先接入 firewalld 的 D-Bus
  API；没有 firewalld 时再使用拥有明确 table/chain 所有权、支持原子更新和回滚的 nftables provider。
  不应无条件直接写 nftables，也不应把 UFW CLI 当成所有发行版通用 API。
- 两个平台都需要各自的特权 helper、规则所有权标识、权限请求、后端冲突检测和实机验收。检测不到可安全管理
  的后端时返回 `manual-required`，不是返回 `ready`，也不是扩大规则范围。

因此“其它系统不能自动创建”是不准确的；准确说法是“可以自动创建，但必须按系统防火墙后端分别实现”。
未来若实现 macOS/Linux provider，应遵循同一个 `FirewallProvider` 抽象，不能把 Windows 规则代码条件编译后
直接复制过去。

当前 Desktop 正式交付仍以 Windows 为优先。非 Windows 的 `server-mode` 可以进入发行包，但必须由
`RuntimeCapabilities` 显式表达防火墙 provider 的具体能力：服务模式能力和“防火墙自动管理能力”分开，不能用
一个 `serverMode=true` 暗示所有平台都已具备自动放行。

### macOS/Linux provider 设计

三个 provider 都应实现同样的最小接口：

```text
inspect() -> 当前后端、权限和已有 WineStock 规则状态
ensure(port, local_scope) -> 创建/更新当前端口规则
remove(port) -> 删除当前端口的自有规则
reconcile() -> 清理旧端口和孤儿规则
```

macOS provider 以 PF anchor 为首选方案；Linux provider 以 firewalld D-Bus 为首选、nftables 为可控回退。
provider 必须先确认自己拥有对应规则的管理边界，不能删除用户或其它软件的规则。权限被拒绝、后端正在被其它
管理器接管、规则无法原子更新时，都返回 `requires-elevation`、`blocked-by-policy` 或 `manual-required`。

### macOS 签名特权 helper 详解

macOS 的“签名特权”不是给 Tauri 主进程设置一个布尔权限，也不是把主 App 直接以 root 启动。推荐拆成：

```text
WineStock.app（普通用户、签名、公证）
  -> 受限 XPC/Mach IPC
WineStock Firewall Helper（独立签名、launchd 管理、root）
  -> 仅操作 com.winestock.desktop.server-mode PF anchor
```

#### 1. 签名链

主 App、helper 和安装包必须使用同一 Apple Developer Team 的 Developer ID Application 身份签名，并一同
完成 hardened runtime 和 notarization。helper 需要单独作为 Mach-O 可执行文件签名，不能只依赖外层
`WineStock.app` 的签名；签名要求应固定为同一 Team ID、bundle identifier 前缀和正式发布证书。

签名的作用是让 macOS 在安装、launchd 启动和 IPC 身份校验时确认“这个 root 代码确实来自 WineStock
发布者”。签名本身不会授予 root 权限，也不会绕过管理员授权；helper 仍必须通过系统的特权安装流程获得
root 身份。

#### 2. 安装与授权

采用 Apple Service Management/Authorization Services 的特权 helper 安装流程（传统实现通常使用
`SMJobBless` 及其 launchd plist；目标 macOS 版本确认后再选择对应的现代 Service Management API）：

1. 首次需要自动防火墙时，主 App 请求安装 helper；系统显示管理员授权/密码界面。
2. 安装器核验 helper 的签名要求、版本和固定安装位置，再交给 launchd 以 root 身份托管。
3. 主 App 不保存管理员密码、不修改 `/etc/sudoers`、不写入长期 sudo token，也不使用已废弃的
   `AuthorizationExecuteWithPrivileges`。
4. 用户拒绝授权时，服务可以继续运行，但防火墙状态为 `requires-elevation` 或 `manual-required`；不能
   把服务状态显示成“局域网已可访问”。

helper 可以是按请求退出的短生命周期进程，也可以是安装后由 launchd 托管的极小 root daemon。若采用常驻
daemon，必须把 IPC 面做得足够窄，并在 helper 内再次验证调用者；不能因为安装了 root helper 就开放通用
shell 或任意 PF 文本执行。

#### 3. IPC 与调用者验证

主 App 到 helper 建议使用受限 XPC/Mach service，而不是 TCP localhost、临时文件或可被其它用户抢占的 Unix
socket。helper 每次请求都应：

- 读取调用者 audit token/代码签名身份，确认是同一 Team ID 和预期 bundle identity；
- 只接受固定操作：`inspect`、`ensure`、`remove`、`reconcile`；
- 只接受经过解析的 `u16` 端口、IPv4/CIDR 地址和固定 scope，不接受任意 PF 配置文本或命令行；
- 对端口、地址数量、CIDR 范围和规则数量设置上限；
- 返回结构化状态和受控错误码，不返回 root 路径、凭据或完整命令内容。

主 App 被替换、签名不匹配、IPC 调用者不是正式 WineStock 或参数校验失败时，helper 应拒绝请求并记录
`blocked-by-policy`/`invalid_request`，不能继续使用上一次参数。

#### 4. 为什么不用 macOS Application Firewall 直接放行

`socketfilterfw` 主要以应用路径为单位允许或拒绝连接。它可以自动化，但规则粒度不是当前 TCP 端口；直接
允许 Desktop 应用可能同时放开该进程未来的其它监听端口。因此它只适合作为“应用级兼容/手动引导”方案，
不适合作为本项目 server-mode 的最小权限默认方案。

需要端口级规则时，helper 应管理 PF 的专属 anchor：

- 不修改 `/etc/pf.conf` 的其它内容，不执行全局 flush；
- 只加载和卸载 `com.winestock.desktop.server-mode` anchor；
- 规则限定 TCP、当前端口、当前 IPv4 局域网 CIDR 和当前本机地址；
- 端口或网卡变化时先生成新规则并校验，再替换旧 anchor 内容；
- 停止、切换模式、卸载和异常恢复时只清理自有 anchor；
- 读取 PF 当前状态，PF 被禁用、anchor 被外部接管或加载失败时返回可诊断状态。

PF 规则文本只能在 helper 内由结构化参数生成；即使底层通过系统 PF 控制工具加载，也不能把
`pfctl` 或任意规则文本暴露给前端。若目标版本或签名/特权安装流程尚未完成，macOS provider 应保持
`manual-required`，不能降级成应用级全端口放行。

#### 5. 生命周期和卸载

- 主 App 首次 apply server-mode 时安装 helper，然后执行 `ensure(current_port)`；两者任一失败都不持久化
  新配置。
- helper 安装成功不等于规则 ready；每次启动、端口变化和网络地址刷新都要检查当前 anchor。
- App 正常退出、停止 server-mode 或切换 remote 时调用 `remove/reconcile`；App 崩溃后下次启动收敛孤儿规则。
- 卸载器必须先调用 helper 清理 anchor，再删除 helper 和 App；清理失败时不能执行全局 PF 清空，并留下
  可诊断的卸载错误。
- helper 更新必须先验证新版本签名和 Team ID，再替换旧版本；旧版本不能接受新协议的未知请求。

因此 macOS 可以做到自动、端口级、最小权限的防火墙管理，但交付物不只是一个 Rust 函数，而是签名、公证、
特权安装、root IPC、PF anchor、升级和卸载整套安全链路。

## 问题原因

一次跨设备访问需要同时满足以下条件：

```text
服务绑定局域网地址
  -> Windows 防火墙允许目标 TCP 端口的入站流量
  -> 当前网络配置文件匹配规则（Domain/Private）
  -> 路由器没有启用客户端隔离
  -> 访问设备使用了真实 LAN 地址和正确端口
  -> WineStock HTTP 服务正常运行
```

当前代码覆盖服务监听、Windows 首版规则检查/写入、真实 LAN 地址发布和应用 HTTP 健康检查；路由器隔离等外部
条件仍不由 WineStock 控制。Windows 对未经允许的入站连接通常按当前配置文件的默认策略处理，
所以即使 `http://192.168.x.x:<port>/api/health` 在 Desktop 本机可用，另一台设备仍可能超时。

以下问题不属于同一层，不能用添加防火墙规则替代：

- 端口被其它进程占用：属于 `port_in_use`，应在服务绑定阶段处理。
- Wi-Fi 访客网络或 AP 客户端隔离：属于网络设备策略，Windows 规则无法解除。
- 公网、VPN 或跨路由访问：不属于首版 Desktop server-mode 的局域网范围。
- HTTP 明文和业务认证：属于应用安全边界，开放防火墙不会提供 TLS 或身份认证。

## 目标与非目标

### 目标

- server-mode 启动后，其它同一可信局域网设备可以访问当前实际 TCP 端口。
- 规则只允许入站 TCP，不开放 UDP、出站流量或任意端口。
- 规则只匹配 Domain/Private 配置文件和 `LocalSubnet`，不自动开放 Public 配置文件。
- 规则有稳定的 WineStock 所有权标识，端口变化、停止、切换远端和卸载时可安全清理。
- UAC 取消、组策略拒绝、Firewall 服务关闭和 Public 网络分别反馈可理解的状态。

### 非目标

- 不把 Desktop 进程永久运行在管理员权限下。
- 不自动把 Windows 网络从 Public 改成 Private。
- 不修改路由器、防火墙之外的安全软件或企业组策略。
- 不用“允许该 exe 的所有入站端口”替代端口规则。
- 不把 `0.0.0.0` 或 `::` 作为访问 URL 返回给前端。
- 不因为防火墙规则而改变 Android 或无头 `server` shell 的行为。

## 所有权与架构

```text
frontend
  -> Shell Bridge：读取 firewall 状态、显示确认和错误

desktop 主进程（非管理员）
  -> DesktopRuntimeManager：绑定 core、决定何时需要规则、刷新快照
  -> FirewallCoordinator：校验端口/配置文件并调用固定 helper
  -> UAC runas helper（管理员、短生命周期）
       -> Windows Firewall COM：INetFwPolicy2 / INetFwRules / INetFwRule

winestock_core
  -> 只负责 bind、HTTP 服务和优雅关闭
```

防火墙协调器属于 `desktop`，因为它是 Windows OS 集成和 Desktop 生命周期的一部分。`core` 不应依赖
`windows`、`windows-sys` 或任何 Tauri API。`lan_access.rs` 使用跨平台 `if-addrs`；防火墙 COM 实施统一复用
高层 `windows` crate 的 `Win32_NetworkManagement_WindowsFirewall` feature。

## 规则设计

### 规则字段

建议创建一条由 WineStock 独占管理的入站规则，字段固定如下：

| 字段 | 值 | 目的 |
| --- | --- | --- |
| `Name` | 稳定的 WineStock 专属名称，不能包含易变端口 | 用精确名称识别自有规则 |
| `Grouping` / 描述标识 | 稳定的 WineStock 标识 | 防止清理时误删其它软件规则 |
| `Direction` | `In` | 只处理入站连接 |
| `Action` | `Allow` | 放行符合条件的连接 |
| `Protocol` | `TCP` | WineStock HTTP 服务只需 TCP |
| `LocalPorts` | 当前 server-mode 实际端口 | 不开放其它端口 |
| `Profiles` | `Domain \\| Private` | 不对 Public 网络自动开放 |
| `RemoteAddresses` | `LocalSubnet` | 只允许本地子网来源 |
| `Enabled` | `true` | 规则创建后立即生效 |
| `EdgeTraversal` | `false` | 不允许边缘穿越/NAT 穿透语义 |
| 程序路径 | 不设置为唯一限制 | 避免把端口规则误变成 exe 全端口规则 |

规则应为空 `Interfaces`/接口类型（即按 Windows 默认接口范围工作），由 `LocalSubnet` 限制远端范围。
如果实现阶段确认 `LocalSubnet` 在目标 Windows 版本上的行为不足以覆盖产品需要，再单独评估按当前
`lanAccessUrls` 生成 `LocalAddresses`；不能为了绕过网络变化而直接改成 `RemoteAddresses=Any`。

### IPv4 与 IPv6

当前 Desktop 首版只发布 RFC1918 IPv4 的 `lanAccessUrls`。因此防火墙自动放行首版也应明确限定为 IPv4
server-mode：

- `0.0.0.0` 绑定使用 IPv4 LAN 规则。
- 具体 IPv4 绑定使用同一端口和 LAN 规则。
- `::` 或具体 IPv6 绑定暂不自动创建规则，返回稳定的 `firewall_ipv6_unsupported` 或
  `profile-unsupported` 类错误，并在设置页说明需要 IPv6 专项设计。

这样可以避免用户只看到 IPv4 地址，却因为一个覆盖 IPv6 的宽泛规则意外开放 IPv6 服务。未来若要支持 IPv6，
应增加 IPv6 地址发现、URL 展示、scope 处理和独立验收，不应在本任务中默认放开。

### 规则所有权与清理

不能按“所有名字包含 WineStock 的规则”删除。协调器只操作同时满足以下条件的规则：

1. `Name` 等于代码中的稳定规则名；
2. `Grouping` 等于稳定的 WineStock 规则标识；
3. 新增/更新时规则属于本应用定义的 server-mode 规则形状。

端口变化时更新同一条自有规则；如果新端口授权失败，旧端口规则仍会保留，服务和配置仍可继续使用，前端提示
局域网访问可能不可用。离开 server-mode 时按稳定名称和分组清理自有规则，不依赖当前配置端口，避免这种失败路径
留下旧端口规则。正常停止、重启、异常恢复和应用退出时保留已授权规则；切换到 remote/self-hosted 或卸载时才删除
自有规则。异常崩溃后下一次启动只读检查规则，不自动触发 UAC。

卸载器也应删除同一稳定标识的规则，但只能删除 WineStock 自有规则；不能清空整机防火墙策略。

## 提权方案

### 推荐流程

1. Desktop 以普通用户启动并完成配置校验。
2. `DesktopRuntimeManager` 先按现有流程启动 core 并取得实际绑定端口；不使用用户输入直接拼接命令。
3. 若 server-mode 需要创建或更新规则，Desktop 通过固定路径启动 WineStock Firewall Helper，使用
   Windows `ShellExecuteW`/`ShellExecuteExW` 的 `runas` verb 触发 UAC。
4. Helper 只接受固定动作（`ensure`、`remove`、`reconcile`）和经过严格解析的端口/请求参数，调用
   `INetFwPolicy2` 获取策略，使用 `INetFwRules`/`INetFwRule` 创建、更新或删除规则，然后返回受控退出码。
5. Desktop 等待 helper 退出，把退出码和 Windows HRESULT 映射为稳定 Shell Bridge 状态；helper 立刻结束，
   不作为常驻管理员服务。
6. 服务启动成功后即可持久化新的 server-mode 配置；规则成功时发布 `firewall=ready`，UAC 取消或规则更新
   失败时保留服务并发布对应防火墙状态，由前端让用户选择继续或重试。

`INetFwPolicy2`、`INetFwRules` 和 `INetFwRule` 是 Windows Firewall COM API；`NetFwPolicy2` 和
`NetFwRule` 是对应的 COM 类标识。实施时应使用高层 `windows` crate 生成的 COM 接口、`BSTR`/Windows
字符串类型、`windows::core::Result` 和 HRESULT 错误传播，不手写 vtable 或裸指针封装，也不把 PowerShell
或 `netsh` 的字符串拼接作为产品核心路径。`windows-sys` 不参与 Firewall COM 调用。

### 为什么不让主程序直接提权

- Tauri UI、WebView 和业务服务不需要管理员权限。
- 主程序一旦整体提权，会扩大前端输入、第三方依赖和业务代码的安全影响面。
- 独立 helper 可以做最小参数校验、单独签名和单独审计，也更容易测试 UAC 取消和策略拒绝。

如果第一阶段为了降低交付成本暂时不拆 helper，也必须把防火墙写入封装在 Desktop 的独立模块中，明确记录
这是过渡方案；不能把通用 `Command`、任意 PowerShell 或任意 `netsh` 参数暴露给前端。

## 生命周期与失败语义

### 启动、应用和重启

server-mode 的一次显式应用应按以下事务边界执行：

```text
校验配置
  -> 启动/绑定 core
  -> 发现可发布的 IPv4 地址
  -> 显式 ensure 当前端口的防火墙规则
  -> 规则成功：保存配置、发布 running + ready
  -> UAC/规则失败：仍保存配置并发布 running + requires-elevation/error，前端选择继续或重试
```

从旧的本地服务切换端口时，新规则成功后再清理旧规则。若新服务或新规则失败，应尽力恢复旧服务、旧端口
和旧配置；防火墙失败不属于 core 启动失败，前端必须看到“已应用但局域网访问可能不可用”的防火墙状态。

切换到非 server-mode 时删除自有规则失败，不应阻止用户切换模式；应返回 `cleanup_pending`，并在快照中提示
规则清理未完成。正常停止和退出不执行删除，因此不会因为进程重启反复触发 UAC。

### 不同失败的语义

| 情况 | 推荐服务行为 | Shell Bridge 状态/错误 |
| --- | --- | --- |
| 用户取消 UAC | 保存配置并保留 core，提示局域网可能不可达；用户可继续或显式重试 | `firewall_authorization_required` / `requires-elevation` |
| 组策略或权限拒绝 | 保存配置并保留 core，不伪装成端口冲突；允许继续使用并显示状态 | `firewall_policy_blocked` / `blocked-by-policy` |
| Windows Firewall 服务关闭 | 可运行但明确展示风险；不能显示为“防火墙已保护” | `disabled` |
| 当前配置文件仅为 Public | 默认不自动开放，提示切换为 Private 或手动配置 | `profile-unsupported` |
| COM/API 或 helper 异常 | 保存配置并保留 core，提示局域网可能不可达；显式操作时可重试 | `firewall_rule_update_failed` / `error` |
| 当前网络地址暂时消失 | 保持现有服务状态，刷新 LAN URL；网络恢复时重新收敛规则 | `ready` 或 `error`，不能保留陈旧 URL |
| 用户/安全软件外部删除规则 | 不重复弹 UAC 循环；显示外部设备可能不可达，显式重试时重新授权 | `requires-elevation` 或 `error` |

对于已经持久化且自动恢复的配置，启动只调用无权 `probe` 读取规则并刷新状态，不触发 UAC；只有显式保存、
端口变化、模式切换或用户点击“重试防火墙”时才调用 `ensure/remove`。应用焦点恢复可以无权读取规则，
但不应每次获得焦点都弹 UAC。

## Shell Bridge 契约建议

现有 `RuntimeServiceSnapshot` 增加一个可选的防火墙对象，不把状态塞进普通服务错误字符串：

```json
{
  "firewall": {
    "status": "ready",
    "port": 17890,
    "profiles": ["domain", "private"],
    "scope": "local-subnet"
  }
}
```

建议的 `status` 枚举为：

- `not-required`：remote、self-hosted、stopped 或非 Desktop server-mode；
- `ready`：规则存在且关键字段匹配当前端口；
- `manual-required`：当前平台可以启动服务，但需要用户/管理员在系统防火墙中手动放行；
- `unsupported`：当前平台或当前绑定协议没有可用的防火墙适配器；
- `requires-elevation`：尚未授权、UAC 被取消或规则需要管理员修改；
- `blocked-by-policy`：组策略/权限阻止本地修改；
- `profile-unsupported`：当前网络是 Public，规则未按 Public 开放；
- `disabled`：Windows Firewall 服务或防火墙功能关闭；
- `error`：未知 COM、helper 或规则读取错误。

快照中的 `ready` 只表示规则匹配，不表示路由器一定允许访问。前端应继续展示实际 `lanAccessUrls`，并在
`ready` 之外明确区分“服务运行”和“局域网访问条件”。现有 server-mode 文案“允许其他设备连接”可以保留，
但保存前应补充“将请求 Windows 防火墙为本机局域网 TCP 端口放行”的说明；UAC 取消、Public 网络和防火墙关闭
不能都显示成同一句“请检查防火墙”。

建议新增稳定错误码：

```text
firewall_manual_configuration_required
firewall_unsupported_platform
firewall_authorization_required
firewall_policy_blocked
firewall_profile_unsupported
firewall_ipv6_unsupported
firewall_service_unavailable
firewall_rule_update_failed
firewall_cleanup_pending
```

这些错误码属于 Desktop Shell Bridge，不是 core HTTP 错误，也不需要修改 OpenAPI 或运行服务 API。

## Windows UI 调整

需要调整 UI，但不需要新增独立的“防火墙设置页”。现有运行设置页、server-mode 确认 Dialog 和 LAN 地址
Dialog 足够承载这项能力。

### 必须修改

1. **server-mode 说明**：把“同一网络中的其他设备将能够连接此服务”改成有条件的表述，例如“应用会请求
   Windows 防火墙允许当前端口的局域网连接；仅对域网络和专用网络生效，系统可能显示权限确认”。不能在
   规则尚未创建前承诺一定可达。
2. **保存确认 Dialog**：显示当前端口、允许范围和 UAC 目的。明确说明这次授权是 Windows 防火墙权限，和
   WineStock 内部的管理员密码不是一回事；HTTP 局域网访问仍只适用于可信网络。
3. **运行状态卡**：server-mode 下增加防火墙状态行，至少区分：
   - `ready`：Windows 防火墙已允许当前端口的局域网访问；
   - `requires-elevation`：需要重新确认 Windows 权限；
   - `blocked-by-policy`：组织策略或安全软件阻止修改；
   - `profile-unsupported`：当前是 Public 网络，不会自动开放；
   - `disabled`：Windows Firewall 未启用，不能表示为“已保护”；
   - `error`：规则读取/更新失败。
4. **地址提示拆分**：没有 `lanAccessUrls` 时提示网络适配器或绑定地址不可用；有地址但 firewall 非 `ready`
   时提示“地址存在，但 Windows 防火墙尚未允许连接”。不能继续使用“请检查网络适配器和操作系统防火墙”
   这一条混合文案覆盖所有情况。
5. **失败后的重试**：UAC 取消或策略拒绝后保留用户的 server-mode 草稿和稳定错误，提供“重新授权/重试”
   入口。推荐新增具名 `shell_repair_firewall` command；如果实现阶段确认重新 apply 同一配置可以可靠触发
   `ensure`，也可以复用已有 apply 流程，但不能要求用户修改无关配置才能重试。

### 可以复用

- 现有 server-mode 强制设置 WineStock 管理员密码的流程继续保留；它解决的是远端登录凭据，不替代 Windows
  UAC。
- 现有 `LanAccessDialog` 继续只展示 Shell 返回的真实 URL，不在前端生成地址或判断防火墙。
- 现有页面错误区和 Notice 用于呈现稳定 Shell 错误，不弹第二套自定义原生设置窗口。

### Windows-only 发布策略

当前版本只实现 Windows 自动防火墙 provider。非 Windows 构建应由 Shell capability 禁用 server-mode，或明确
返回 `manual-required`；前端沿用已有 capability disabled reason，显示“当前版本仅支持 Windows 自动配置局域网
访问”，不能让用户进入一个没有可执行授权路径的 server-mode 表单。

### 不需要修改

- 不改变“允许其他设备连接”的运行模式名称和基本配置字段；
- 不把防火墙规则字段暴露成普通表单；
- 不新增端口扫描、网卡枚举或业务 API；
- 不把 UAC 凭据、Windows 用户名或规则内部错误详情展示给前端。

## 安全边界

- 规则只开放当前端口；端口必须是已通过 `u16`/`1..65535` 校验的数值，不接受任意原始命令参数。
- 默认只允许 `Domain|Private` 和 `LocalSubnet`，不包含 Public；不自动修改网络配置文件。
- Helper 的可执行文件应和应用使用同一发布者签名，安装到受控应用目录；启动时校验自身路径和固定参数格式。
- Helper 不读取或返回数据库、认证 token、绝对存储路径或业务请求内容。
- 日志只记录状态、端口和错误码，不记录 UAC 凭据或完整命令行敏感内容。
- 规则名称和分组所有权标识必须是产品常量；删除前按固定名称和分组精确匹配。
- 不允许前端自定义规则名称、程序路径、远端地址、Public profile 或任意 PowerShell 代码。
- HTTP 局域网访问仍需依赖现有首次设密门和业务认证；防火墙规则不是认证机制。

## 实施步骤

### 第一阶段：规则与桥接模型

- 将 `desktop/src/lan_access.rs` 从当前 Windows-only IP Helper 实现迁移到 `if-addrs 0.15.x`，统一
  Windows/macOS/Linux 的接口枚举；迁移完成后删除仅为地址发现引入的 `windows-sys` 依赖和对应 unsafe 链表解析。
- 在 `desktop` 的现有 `windows` 依赖上增加 `Win32_NetworkManagement_WindowsFirewall` feature，并增加独立
  `firewall` 模块；不修改 `core`/`shared`，也不为 Firewall API 新增 `windows-sys` 依赖。
- 定义规则常量、状态枚举、稳定错误码和 Shell Bridge 可选 DTO。
- 为规则构造器写纯 Rust 测试，覆盖端口、Profile 位掩码、`LocalSubnet`、TCP、入站和禁止 Public。
- 明确首版 IPv4 限制，并把 IPv6 行为接入配置校验或可见错误。

### 第二阶段：helper 与 COM 操作

- 实现 `ensure/remove/reconcile` 三个固定动作。
- 通过 `INetFwPolicy2` 查询当前 Profile、Firewall 开关和本地策略修改状态。
- 通过 `INetFwRules` 精确枚举 WineStock 自有规则，新增/更新当前端口并清理过期端口。
- 使用 `ShellExecuteW` 的 `runas` 路径覆盖 UAC 接受、取消和拒绝。
- 将 helper 作为受控 sidecar/安装资源打包，并在 Windows 发布构建中验证签名和路径。

### 第三阶段：RuntimeManager 生命周期接入

- 显式保存 server-mode 在成功 bind 后调用 `ensure`；规则失败时保留新服务和配置持久化，并发布防火墙告警。
- 软件启动和应用恢复时只调用无权 `probe`，规则不符合条件时通过前端提示是否重新设置。
- 防火墙规则是 Windows 的持久配置：正常停止、重启、异常恢复和进程退出只停止 core、释放端口，
  不删除已授权规则；下次启动先只读检查，匹配时不触发 UAC。
- 从 server-mode 切换到 remote/self-hosted 时删除自有规则；server-mode 内部端口变化由新的 `ensure`
  更新规则，通常只在规则需要变更时再次请求 UAC。
- 将只读状态刷新接入快照和焦点恢复；禁止无感知反复弹 UAC。
- 保持现有 `lan_access.rs` 的职责：它只发现访问地址，不写防火墙规则。

### 第四阶段：前端与安装器

- 在保存 server-mode 前显示局域网范围、端口和 UAC 目的说明。
- 为 `ready`、`requires-elevation`、`blocked-by-policy`、`profile-unsupported` 和 `disabled` 提供不同文案。
- NSIS 卸载阶段清理带稳定 WineStock 标识的规则；卸载失败不能删除其它规则，并应有可诊断日志。
- 补充手动防火墙配置入口或文档，供企业策略禁止应用写本地规则时使用。

## 验收矩阵

### 自动化与本机测试

- 规则字段单元测试：端口变化、Profile 不含 Public、远端范围为 `LocalSubnet`、Edge Traversal 关闭。
- helper 测试：非法动作、非法端口、重复 ensure、remove 幂等、只清理自有规则。
- RuntimeManager 测试：启动 probe 不触发 helper；显式 ensure 失败时配置和 core 仍可保留；规则成功后快照和配置一致；
  模式切换清理失败不阻止切换，且可通过 repair 重试。
- Shell Bridge 契约测试：新字段可选，旧快照仍可反序列化，稳定错误码不退化为 `service_start_failed`。

### Windows 实机/虚拟机

- 防火墙开启、Private 网络：同网第二台设备访问 `http://<lan-ip>:<port>/api/health` 成功。
- 首次授权接受 UAC：规则出现，端口和 Profile 字段正确。
- UAC 取消：服务和配置保留，前端提示局域网可能不可达，并提供继续使用/重试入口；启动检测不自动弹 UAC。
- Public 网络：不创建 Public 放行；状态为 `profile-unsupported`，手动切换 Private 后可修复。
- Group Policy 拒绝：状态为 `blocked-by-policy`，不循环弹窗，不删除其它规则。
- 防火墙服务关闭：服务行为、风险提示和状态与规则 ready 明确区分。
- 端口变更：新端口可访问，旧端口规则清理；中途失败能恢复旧配置。
- 正常停止、正常退出和异常重启：规则保持；切换 remote/self-hosted：显式清理并在失败时提供重试；卸载流程最终清理自有规则。
- 访客 Wi-Fi/AP 客户端隔离：防火墙规则正确但访问仍失败，诊断结果应指向网络设备而非继续扩大规则。
- IPv6 绑定：按首版策略明确拒绝自动放行或显示不支持，不产生未验证的 IPv6 暴露。

## 技术依据

实现阶段以目标机器的 Windows SDK 和锁定的 Rust crate 版本为准，主要 API 参考：

- [INetFwPolicy2](https://learn.microsoft.com/en-us/windows/win32/api/netfw/nn-netfw-inetfwpolicy2)
- [INetFwRules](https://learn.microsoft.com/en-us/windows/win32/api/netfw/nn-netfw-inetfwrules)
- [INetFwRule](https://learn.microsoft.com/en-us/windows/win32/api/netfw/nn-netfw-inetfwrule)
- [ShellExecuteW](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecutew)，其中 `runas` verb 用于触发 UAC
- [Rust for Windows](https://github.com/microsoft/windows-rs)，当前 Context7 解析 ID 为 `/microsoft/windows-rs`
- [if-addrs 0.15](https://docs.rs/if-addrs/0.15.0/)，用于跨平台高层网卡地址枚举

当前 Context7 查询到 Rust for Windows 的高层 `windows` crate 支持通过生成的 COM 接口、`BSTR` 和
`windows::core::Result` 管理 Windows API；本实现已通过 Windows 目标编译验证
`INetFwPolicy2`/`INetFwRules`/`INetFwRule` 的实际可用性。`windows-sys` 不参与地址发现或 Firewall API。
