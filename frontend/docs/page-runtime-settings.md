# 本机运行设置 Dialog

本文记录运行设置的当前实现边界。运行设置是 Desktop、Android 和 Web fallback 共用的唯一运行配置 UI，正常已进入应用时由 AppShell 以 Dialog 打开；平台 Shell 不提供原生设置窗口、Activity 或对话框。

`/settings/runtime` 路由仍然保留，但只承担首次配置、认证前入口、服务恢复和无应用壳状态。它挂载的仍是同一个 Dialog 组件，不再维护第二套表单或页面布局。

## 可用性边界

- 路由声明 `requiresAuth = false`、`requiresService = false`。
- 配置缺失、配置损坏、API 未启动、端口占用或远端断连时仍必须完整渲染。
- **设置未完成**时的守卫分流：首次 `unconfigured` 且未初始化走 `/setup` 初始化向导；`invalid`、已配置但服务未就绪等其余未就绪状态强制进入本页。`returnTo` 保留原目标。
- **设置已完成**（Shell initialized 且可 HTTP）时不强制进入；可从账户弹层、认证页、服务不可用覆盖层等侧入口打开。
- 服务不可用覆盖层不得遮挡本页。
- 首次未初始化时不要求或伪造 `apiBaseUrl`；Shell 只提供默认表单草稿，用户 apply 后才发布实际地址。

## server-mode 强制设密门

本机静默免登录会话（`localSilentAuthActive`）下把运行方式切到「局域网服务器」提交时：

- 先经已鉴权接口 `GET /api/auth/local-session/status` 查询占位密码标记；
- 仍为占位（自动开通的随机密码，无人可输）时弹出「先设置当前用户密码」Dialog，同时填写当前用户名和新密码，
  经免旧密码通道 `POST /api/auth/me/password`（`current_password` 留空）设置真实凭据后
  才回到正常的确认与保存流程；取消 Dialog 则本次不保存。
- `username` 是该接口的必填请求字段；成功后重新请求 `/api/auth/me` 同步当前会话用户名。
- 状态查询失败时阻止提交并提示，避免带着占位密码开放局域网（届时局域网端无人能登录）。
- 设计见 `docs/implementation-notes/self-hosted-silent-auth.md`。

## 启动漏斗中的位置

界面顺序为：**运行设置 → 统一认证入口 `/auth` → 注册/登录 → 业务**。

- **`initialized` 仅由 Shell 在「保存设置」成功后发布为 true**：首次即使表单未改也可点保存；未保存前不展示「继续」。
- 保存成功且匿名时自动进入 `/auth`；已确认后页头为「继续」/「← 返回应用」。

## 运行方式

页面提供三个用户可见选项：

- 本机运行：使用 `127.0.0.1`，首次 apply 或端口冲突时由 Shell 自动选择并持久化实际值；已有配置的应用启动时自动运行本地服务；页面不展示端口或监听地址输入框。
- 连接远程服务：编辑完整 HTTP/HTTPS API 根地址，本设备不管理本地服务。
- 局域网服务器：本机启动服务并允许其它设备连接；固定端口和监听地址收纳在默认收起的“高级设置”中，是否可选由 Shell capability 决定。
- Android 上“局域网服务器”会明确显示当前只能使用本机 `127.0.0.1` 的原因，不会伪装成普通禁用态。
- 纯网页端（快照 `platform === "web"`）只有连接远程服务一种能力：“本机运行”和“局域网服务器”均禁用并显示“浏览器无法在本机启动服务”的原因；本机类持久配置在进入页面时被动纠正为远端草稿（须保存才生效）。与初始化向导在 web 下跳过使用方式页的语义一致（见 `docs/implementation-notes/first-run-setup-wizard.md`）。

`bindHost`、`port` 和 `remoteBaseUrl` 形成可编辑 DTO。自动启动不是配置项：本地模式在配置 initialized 后由 Shell
固定映射为 `auto_start_server = true`，首次缺少配置时等待 apply；远端模式不启动本地服务。前端不读写平台配置文件，
也不根据 `bindHost` 自行生成对外访问地址；实际 `apiBaseUrl`、监听地址和局域网 URL 由 Shell 快照返回。

## 交互规则

- 当前服务区只展示 Shell 发布的生命周期状态和 `/api/health` 结果，不提供手动启动、停止或重启 core 的操作；本机服务由应用 Shell 管理。
- 运行设置页不再提供“查看局域网访问地址”按钮；地址入口由已登录应用壳的头像 Popover 提供，避免重复放置入口。
- 当前服务满足 `server-mode + local + running + serverMode capability` 但 Shell 没有返回可用地址时，设置页只显示网络适配器/
  监听地址提示；不生成占位 URL，也不显示无法打开的地址入口。
- 头像 Popover 使用“本机局域网地址” Dialog。地址逐项使用线性复制图标按钮并通过全局 Notice 反馈，Dialog 复用通用遮罩、
  焦点、Escape、关闭动画和原生返回处理。
- 前端只规范化、过滤和去重 Shell 地址；wildcard、loopback、占位符、带凭据、路径、查询或片段的值不得展示。
  快照不再满足展示条件时，入口隐藏并关闭已经打开的 Dialog。
- 服务器模式端口必须是 `1..65535` 的整数；本机模式的临时 `port = 0` 不能生成伪访问地址，绑定成功后只展示 Shell 返回的真实地址。
- 远端 HTTP 连接仅在非 loopback 地址上提示明文通信风险；`localhost`、`127.0.0.1` 和 `::1` 不提示远端网络风险。
- Windows Desktop 的 server-mode 保存确认必须说明会请求 Windows 防火墙允许当前端口的局域网连接；这不是
  WineStock 当前用户密码，且只对 Domain/Private 配置文件和本地子网生效，不自动开放 Public 网络。
- Windows Desktop 在保存 server-mode、切换端口或切换运行方式时显式处理防火墙；UAC 取消后配置和服务仍可继续，
  页面提示局域网可能不可达，并提供“继续使用/重试”。软件启动只读检查规则，不自动触发 UAC；不符合条件时打开
  同一恢复 Dialog。重试调用独立的 `shell_repair_firewall`，不重启 core。系统策略阻止、Public 网络配置文件和
  防火墙关闭分别显示独立状态，不把它们伪装成 core 服务启动失败。
- server-mode 的服务状态必须把 core 生命周期和 Windows 防火墙状态分开呈现：`ready`、`requires-elevation`、
  `blocked-by-policy`、`profile-unsupported`、`disabled` 和 `error` 不得都显示为“服务启动失败”或“请检查防火墙”。
- 没有 LAN URL 与防火墙未放行是两类不同问题：前者提示网络适配器/绑定地址，后者提示规则授权或系统策略；
  不能只用一条混合错误文案。
- 远端“测试连接”只验证当前可达性；暂时不可连接不阻止保存格式有效的地址。
- API 地址变化、运行方式变化或开启局域网监听时必须先确认。切换服务后清理旧服务的内存会话并重新执行健康检查和会话初始化；
  Desktop 本地 `server-mode` 仅切换端口时仍是同一台服务，保留当前登录会话。
- 本地服务激活失败时保留草稿和 Shell 稳定错误，不用原生对话框替代页面反馈。
- 底部“取消”始终复用页头“返回应用”的离开目标，不负责仅恢复草稿；已完成设置时可随时返回，首次未完成设置仍由初始化守卫阻止离开。

## Web fallback

普通浏览器使用 `winestock.runtime.config.v1` 保存已应用配置。没有环境地址和持久配置时返回 `unconfigured` 快照及默认 `17890` 草稿；损坏或校验失败的记录返回 `invalid` 快照，让用户在本页修复。

Web fallback 不具备真实本地服务生命周期和网卡发现能力，不得生成占位 `lanAccessUrls`。页面只能依据
快照展示服务状态和真实地址，不能从 User-Agent、全局对象名称或运行目录猜测能力。

## 布局与响应式（2026-08-02 Dialog 迁移后）

正常入口使用共享 `ModalDialog`：桌面使用宽 Dialog，移动端由通用 Dialog 变为底部面板；
标题、关闭按钮和底部操作栏由 Dialog 外壳统一提供。取消、Escape、遮罩和原生返回都执行同一个离开规则。
运行方式为**全宽度统一的分段 tab**（沿用项目既有分段选择模式），配置区随选中 tab 切换。

首次配置和服务恢复虽然由 `/settings/runtime` 路由挂载，但仍使用相同 Dialog 外壳，保证所有入口的焦点、关闭、滚动和嵌套授权行为一致。

- 状态卡按 tone（成功/警告/危险/中性）使用 token soft 底色 + 实色圆点；
  **本机/局域网模式不展示回环地址**（实现细节，对用户无意义），仅远端模式展示服务器地址。
- 整页在桌面与 `390 × 844` 均不出现页面滚动；窄屏收紧 tab 内边距与字号保证三个 tab 单行。
- 纯网页端两个本机类 tab 置灰（title 提示原因）并在 tab 下方显示一行说明；
  本机类历史配置进入页面时被动纠正为远端草稿，`remoteBaseUrl` 为空时用当前生效服务地址预填。

## 验收要点

- 至少验证 `1440 × 900`、接近 `768px` 和 `390 × 844`；均不得产生页面或 Dialog 横向溢出。
- Web 未配置（`unconfigured`）走 `/setup` 向导；`invalid` 或已配置未就绪进入本页修复。
- 本机服务自动分配并持久化实际端口、保存操作可用，且没有鉴权或断连覆盖层。
- 匿名完成/确认设置后进入 `/auth`，不得直达 `/login`。
- 本机局域网地址 Dialog 需检查多条 IPv4/IPv6、长地址、复制反馈和地址失效关闭。
