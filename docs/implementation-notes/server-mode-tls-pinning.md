# server-mode 自签 TLS 与首次信任固定（2026-07-27 设计定稿）

本文记录 server-mode 从明文 HTTP 迁移到自签名 HTTPS + 客户端指纹固定的跨
core/契约/Android/frontend 设计与实施细节。
当前状态：**设计定稿、未实施**——作为后续任务的执行依据，粒度按"拿起即可开工"编写。
已确认决策：HTTPS-only、TOFU 首次信任（扫码配对为后续增强）、先文档后实施。

## 威胁模型与目标

- 使用面：家庭/小作坊局域网，server-mode 设备服务同网段的壳客户端与少量浏览器。
- 现状风险按严重度：登录请求明文密码 > Bearer token 被嗅探 > 业务数据被旁观；
  攻击前提是内网存在恶意设备（合租网络、访客 Wi-Fi、被入侵的 IoT）。
- 目标：server-mode 传输全程加密，客户端可验证服务器身份；不引入域名、CA、
  外网依赖，不破坏离线可用文化。
- **HTTPS-only**（已确认）：server-mode 不保留明文监听，不做可选开关，不做双监听过渡；
  存量客户端迁移即重新信任一次。self-hosted 回环（127.0.0.1）保持 HTTP 不变。

## 信任模型：TOFU 首次信任 + 严格固定

- 验证锚点是服务器公钥的 **SPKI SHA-256 指纹**。固定公钥而非整证书：
  证书续期不换密钥则客户端无感；换密钥 = 更换身份，必须重新信任。
- 客户端首次连接时取得服务器证书 → 计算 SPKI 指纹 → 弹**信任确认框**
  （展示指纹，供用户与服务器设置页展示值人工核对，核对可选不强制）→
  确认后指纹持久化进运行配置；此后每次 TLS 连接严格比对，
  **失配一律硬失败**，呈现"服务器身份已变化，请重新信任"，任何形式的
  "仍然继续/忽略"入口都不允许存在。
- TOFU 的安全窗口只有首次确认那一瞬（该瞬间若有活跃中间人可被其顶替）。
  后续增强（不在本期）：扫码配对把指纹带外传入，消除首次窗口——
  存储与校验机制完全复用，仅替换指纹录入方式。

### 指纹格式（精确定义）

- 计算：X.509 证书 `SubjectPublicKeyInfo` 的 DER 字节 → SHA-256（32 字节）。
  - Rust：`rcgen::KeyPair::public_key_der()` 即 SPKI DER；
  - Java：`certificate.getPublicKey().getEncoded()` 即 SPKI DER。
- 存储/传输格式（RFC 7469 风格）：`sha256/` + 标准 Base64（44 字符含填充），
  正则 `^sha256\/[A-Za-z0-9+/]{43}=$`。
- 人工核对短格式（仅展示）：摘要前 8 字节十六进制大写，4 字符一组空格分隔，
  如 `3A7F 9C21 08D4 55EE`；确认框与服务器设置页使用同一格式渲染。

## 服务端（core，Rust）

### 依赖与准备

- 新增：`rcgen`（证书生成）、`axum-server`（rustls acceptor，启用 `tls-rustls` feature）。
  rustls/webpki 生态已在依赖树（LCSC TLS 修复引入）。
- 离线构建照例：联网阶段 `cargo fetch --locked` 后普通构建不联网。

### 证书生成与持久化

- 算法 ECDSA P-256（浏览器/WebView 兼容性最稳），有效期 10 年
  （固定校验不看有效期，长有效期只为浏览器手动信任场景少弹警告）；
  SAN 填生成时可枚举的局域网 IP + `winestock.local`（尽力而为，固定校验不依赖 SAN）。
- 文件落 storage 数据目录子目录 `tls/`：`server_identity_key.pem` + `server_identity_cert.pem`；
  Unix 下密钥文件权限 0600，Android 应用私有目录天然隔离。
- 加载序：文件存在且可解析 → 复用；缺失/损坏 → 重新生成并落盘
  （损坏视同重置身份，属可接受语义：客户端将失配并重新信任）。
- 仅 server-mode 需要证书；self-hosted 启动不触发生成。

### 监听分流与接入点

- 现有装配：`core/src/server.rs` `bind_server()`（约 L125）绑定 TcpListener，
  `serve_local_with_shutdown()`（约 L44）`axum::serve(listener, router)`。
- 改动：`ServerConfig.uses_local_service()` 分支内按 mode 分流——
  server-mode 走新增 `serve_local_tls_with_shutdown(tls_identity, ...)`：
  tokio listener `into_std()` 后交 `axum_server::from_tcp_rustls(std_listener, RustlsConfig::from_pem(...))`；
  graceful shutdown 用 `axum_server::Handle::graceful_shutdown`，由现有 `shutdown_rx`
  触发（spawn 一个等待 shutdown_rx 后调 handle 的任务），对外语义与现有 HTTP 路径一致。
- self-hosted 保持现有 `axum::serve` HTTP 路径零改动。

### 状态透传

- `NativeServiceState`（android/native contract）与 server shell 状态输出新增
  `tlsFingerprint?: string`；`lanAccessUrls` 生成协议按模式切换为 `https://`。
- `RunningLocalService`/`LocalServiceBootstrap` 携带指纹供状态查询。

### 重置服务器身份

- **用途**：私钥疑似泄漏（设备丢失/借出/疑似被入侵后恢复），或一次性作废所有
  已配对设备的信任。普通续期不需要。
- **入口与权限**：只存在于 server-mode 设备**本机**运行设置页（本机指纹展示区旁），
  经 Shell Bridge 本机方法触发；**不做成 core HTTP API**——远端可调的重置会让任何
  admin 会话都能把全部设备踢下线，攻击面与误操作面过大；收敛为物理在场操作，
  与 `stopLocalService` 同类的壳生命周期语义。
- **链路**：危险确认 Dialog（明示"所有已连接设备将需要重新信任"）→ Bridge
  `resetTlsIdentity()` → 壳经 JNI 调 core 新增的 `reset_tls_identity(storage)`
  （删除 `tls/` 下密钥与证书文件）→ 壳走现有 restart 链路重启本地服务
  （重启时按"缺失→重新生成"路径获得新身份）→ 新快照发布新 `tlsFingerprint`。
- **客户端后果**：下次连接指纹失配硬失败 → "服务器身份已变化"错误页 →
  "重新信任"重跑 probe + 确认框（展示新指纹）→ 覆盖保存；旧指纹无残留价值。

## 契约（Shell Bridge v1 加法，不升版本）

```ts
// RuntimeSnapshot.service 追加
tlsFingerprint?: string;        // 本机 server-mode 运行时发布（sha256/Base64）

// EditableRuntimeConfig 追加
remoteCertFingerprint?: string; // 远端模式已信任指纹；缺省/空串 = 未信任（触发 TOFU）

// capabilities 追加
tlsPinning: boolean;

// 可选方法（capability=true 时必须同时存在，语义同 nativeBack 扩展的门控规则）
probeTlsFingerprint(url: string): Promise<{ fingerprint: string; hexPreview: string }>;
resetTlsIdentity(): Promise<RuntimeSnapshot>;   // 仅本机 server-mode 有效
```

- shared 权威校验接受 `remoteCertFingerprint` 可选字段，格式按上文正则；
  仅远端模式允许非空。
- 新稳定错误码：`tls_fingerprint_mismatch`、`tls_probe_failed`。
- `frontend/src/shell/contract.ts` 的快照/配置断言、`cloneRuntimeConfig`、
  web fallback（capability=false、字段透传）、`shell/bridge.js` 方法路由、
  `ShellBridgeHost` 信封分发同步更新。
- Web fallback 语义：纯浏览器无 pinning 能力，`https://` 自签地址依赖用户在
  系统/浏览器层手动信任；前端按 capability 隐藏 TOFU 流程，属二等路径。

## 失配信号送达前端的机制

不新增事件通道，复用快照 `service.error`：

- Android 在 `onReceivedSslError` 判定失配并 `cancel()` 后，向 manager 报告一次；
  manager 发布 remote ownership 快照（phase 维持 `running`）且
  `service.error = { code: "tls_fingerprint_mismatch", message: 稳定文案 }`。
  按"同一失配只报告一次"去重，避免每个被取消的请求都刷新快照。
- 前端：远端模式下 `snapshot.service.error?.code === "tls_fingerprint_mismatch"` 时，
  服务不可用覆盖层与运行设置页优先呈现"服务器身份已变化"专用文案与
  【重新信任】动作（重跑 probe + 确认框，确认后 apply 覆盖指纹并清错误）；
  该状态下常规的"检查网络"文案不得出现。
- 可用性层无需感知该错误码（请求失败自然走既有去抖→unavailable 路径），
  仅呈现层按错误码切换文案与动作。

## Android 壳

### probeTlsFingerprint

- 独立 `SSLContext`（trust-all TrustManager，**仅用于这一次握手取证书，绝不复用于
  业务请求**）+ `SSLSocket` 对目标 host:port 握手，取服务端链首证书 →
  `getPublicKey().getEncoded()` → SHA-256 → 两种格式返回。
- 超时 4s；DNS/连接/握手失败映射 `tls_probe_failed`。仅在用户主动保存地址时调用，
  不做任何自动探测。

### WebView 固定

- `WebViewClient.onReceivedSslError`：
  - 仅处理主因为 `SSL_UNTRUSTED`/`SSL_IDMISMATCH` 的错误，且请求 URL 的 host:port
    与当前远端配置一致；其余一律 `cancel()`。
  - 证书提取：API 29+ 用 `SslCertificate.getX509Certificate()`；minSdk 26 兼容路径
    经 `SslCertificate.saveState()` bundle 的 `x509-certificate` 字节重建。
  - SPKI 指纹 == `remoteCertFingerprint` → `proceed()`；否则 `cancel()` 并按上节
    报告失配。比对逻辑抽成纯类（如 `PinnedFingerprintMatcher`）供 JVM 单测。
- 配置持久化：`RuntimeConfigStore` 增加可选指纹字段；JNI 契约
  （validate/apply/快照 JSON）同步透传；`RuntimeSnapshotFactory` 输出新字段。
- `resetTlsIdentity`：JNI 新增 `nativeResetTlsIdentity`（core 删文件）→ 复用现有
  restart 内部链路；capability 仅在本机 server-mode 配置下为 true。
- `network_security_config` cleartext 放行暂保留（远端 HTTP 仍是合法配置），
  收紧另立任务。

## 前端

- **TOFU 确认流**：向导"连接服务器"页与设置页保存 `https://` 地址、capability
  `tlsPinning` 可用且配置内无指纹时：`probeTlsFingerprint` → 信任确认 Dialog
  （短格式指纹大字展示 + 完整格式次要展示 + "请在服务器设备的运行设置页核对相同指纹"）
  → 确认后把指纹并入草稿随 `applyRuntimeConfig` 保存；取消则本次不保存。
  probe 失败：呈现 `tls_probe_failed` 稳定文案，允许重试，不允许"跳过验证保存"。
- **失配呈现**：见"失配信号送达"一节；文案与动作复用 ServiceUnavailableScreen
  变体机制新增 `remote-tls-mismatch` 语义。
- **服务器侧展示**：运行设置页 server-mode 配置区与"本机局域网地址" Dialog 展示
  本机指纹（快照 `tlsFingerprint`，短格式 + 复制完整格式），旁挂"重置服务器身份"
  危险操作（确认 Dialog 后调 `resetTlsIdentity`）。
- 指纹格式化/校验为纯函数模块（如 `shell/tlsFingerprint.ts`），node 单测覆盖。
- 附带收益记录：LAN 客户端进入安全上下文后浏览器 `getUserMedia` 解锁，
  扫码不再只能走拍照降级（限完成手动信任的浏览器）。

## 兼容与迁移

- 服务器升级后 server-mode 即 HTTPS-only：旧 `http://` 地址全部失效，
  `lanAccessUrls` 即时变 `https://`；各客户端改地址并完成首次信任，一次性成本。
- 旧版本客户端连 HTTPS-only 服务器将连接失败：升级客户端，不做服务端兼容层。
- 远端 HTTP（连接第三方部署的明文服务）本期仍合法，仅保留现有明文警示。

## 明确不做

- 本地 CA / 系统根证书安装方案；HTTP+HTTPS 双监听；任何"忽略证书错误"用户开关；
  HTTP 上自造应用层加密（明文上下文无 `crypto.subtle` 可用，先鸡后蛋）。
- 远程可调的身份重置 API。
- Tailscale / 域名 + DNS-01 属部署文档层建议，不进产品。

## 交付切片

1. **core**：`tls/` 身份管理（生成/复用/损坏重建/reset）+ server-mode rustls 监听分流
   + 状态透传（含 winestock-server shell 的状态打印）；准备命令 `cargo fetch --locked`。
2. **契约与 web fallback**：字段/方法/capability/错误码；`docs/shell-bridge.md` 同步。
3. **Android**：probe、WebView 固定（纯逻辑抽类）、配置与 JNI 透传、reset 链路。
4. **frontend**：TOFU 确认 Dialog、失配变体呈现、服务器侧指纹展示与重置入口、
   `tlsFingerprint.ts` 纯模块。
5. **文档同步 + 真机验收**（清单见下）。
6. （另立任务的后续增强）扫码配对录入指纹；远端 HTTP cleartext 收紧；桌面壳接入。

## 分层测试计划

- core 单测：证书首启生成/重启复用（指纹稳定）/文件损坏重建/reset 后指纹变化；
  server-mode HTTPS 与 self-hosted HTTP 分流；graceful shutdown 语义不回归。
- Android JVM 单测：`PinnedFingerprintMatcher`（匹配/失配/证书链取叶/API<29 路径）、
  `RuntimeConfigStore` 新字段序列化、快照 JSON 新字段。
- frontend node 单测：指纹格式校验/短格式渲染纯函数；契约断言接受新字段。
- 真机验收：
  1. server-mode 启动即 HTTPS，明文端口不存在；重启指纹不变；
  2. 客户端首次保存 https 地址 → 确认框指纹与服务器设置页一致 → 业务全通；
  3. 服务器重置身份 → 客户端硬失败 + 专用文案 → 重新信任后恢复；
  4. 中间人模拟（另一设备以不同证书顶替同地址）→ 拒连、无绕过入口；
  5. self-hosted 回环行为零变化；远端 HTTP 配置仍可用；
  6. 手动信任后的浏览器路径可用且摄像头 `getUserMedia` 可用。

## 文档同步清单

`docs/shell-bridge.md`（契约字段/方法/门控）、`frontend/docs/page-runtime-settings.md`
（TOFU 流程/指纹展示/重置身份/失配呈现）、`docs/implementation-notes/first-run-setup-wizard.md`
（连接服务器页的 TOFU 分支）、`docs/code-map/`（core/android/frontend 三张图）、
`android/docs/README.md`（WebView 固定与 probe）、`docs/implementation-notes/README.md` 索引。

## 开放问题（实施时决策）

- SAN 中多网卡 IP 的枚举与刷新策略（IP 变化不影响固定校验，仅影响浏览器手动信任体验）；
- 失配报告的去重窗口（建议按"配置代次 + 指纹值"去重）；
- 后续扫码配对的二维码内容 schema（地址列表取舍、版本位）与生成层
  （core Rust `qrcode` crate 倾向，届时单独定稿）。
