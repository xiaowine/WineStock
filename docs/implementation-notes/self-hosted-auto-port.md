# 本机模式自动端口实施方案

## 方案结论

本机使用（`self-hosted`）不再向用户展示端口输入框，由 UI Shell 管理本地 Axum 的监听端口。
局域网服务器（`server-mode`）继续显示并要求用户配置端口，因为其它设备需要稳定的访问地址。
远程客户端模式只编辑远端服务地址。

本机模式采用“自动选择、成功后持久化、后续优先复用”的策略：

1. 首次本机启动使用端口 `0` 请求操作系统分配可用端口。
2. 绑定成功后，从实际 `SocketAddr` 取得端口，并把实际端口写回 Shell 配置与 `RuntimeSnapshot.config`。
3. 后续启动优先使用已持久化的实际端口，保持 API 地址稳定。
4. 已持久化端口被其它进程占用时，Shell 再次使用端口 `0` 分配新端口；成功后原子更新配置和快照，并明确发布地址变化。

端口 `0` 只作为 UI 平台本机模式的内部“自动分配请求”，不作为持久化的有效端口，也不允许出现在 `apiBaseUrl`、局域网地址或用户复制入口中。

## 实施结果

- `shared::ServerConfig.port` 已按运行模式校验：`self-hosted` 接受临时值 `0`，`server-mode` 仍要求 `1..65535`。
- `core/src/server.rs` 继续直接绑定配置中的 `SocketAddr`；端口为 `0` 时由操作系统分配，并通过运行句柄返回实际地址。
- Android native 已允许 `self-hosted + port=0` 启动，并返回非零 `boundAddress` 和 `apiBaseUrl`。
- `LocalCoreRuntimeManager` 首次启动使用端口 `0`；绑定成功后把实际端口回写并持久化。已保存端口冲突时只对 `self-hosted` 自动重试一次动态端口。
- Kotlin native 契约会拒绝端口为 `0` 或 `boundAddress` 与 `apiBaseUrl` 端口不一致的 `running` 状态；
  loopback 主机由 Android native 的 self-hosted 配置约束保证。
- 运行设置页面在 `self-hosted` 下不显示端口和监听地址；`server-mode` 仍保留固定端口与高级监听地址。
- 前端不会生成 `http://127.0.0.1:0`；普通 Web fallback 无法管理本地服务，因此会把待持久化的 `port=0` 规范化为兼容默认端口 `17890`。
- 前端 refresh token 仍按 `api_base_url` 匹配。端口被迫变化时沿用既有地址切换流程清理旧服务会话。

## 目标与非目标

### 目标

- 本机模式首次配置不显示端口，也不要求用户理解端口冲突。
- 本地服务启动后，前端始终使用 Shell 发布的实际 API 地址。
- 端口分配、冲突重试、配置持久化和服务状态仍由 Shell 负责。
- 服务器模式保持可预测的固定端口和局域网访问语义。
- Android 冷启动、Activity 重建、前端刷新和服务重启使用同一份端口状态。

### 非目标

- 不改变 `server-mode` 的网络暴露、局域网地址发现或防火墙策略。
- 不让前端直接探测端口、枚举网卡或启动 core。
- 不让 server shell 静默改用随机端口；无头服务仍使用配置文件中的固定端口并报告冲突。
- 不通过原生 Dialog 呈现端口错误；运行设置页面继续承担错误和重试反馈。

## 配置与协议策略

### 推荐的兼容策略

第一阶段沿用现有数值字段，不新增 `portMode` 字段：

- `self-hosted + port = 0`：临时自动分配请求，只能出现在校验、启动和应用事务中。
- `self-hosted + port > 0`：Shell 已选择出的有效端口，允许用于重启和持久化。
- `server-mode + port`：必须是 `1..65535` 的固定端口。
- 远程模式的 `port` 保留当前 DTO 兼容值，不参与远程 URL 解析或绑定。

这样可以保持 Shell Bridge v1 的字段形状，避免一次性迁移所有平台协议。实现时必须在文档和校验代码中明确 `0` 只对 `self-hosted` 有效，不能把 `0` 泛化为所有模式的合法端口。

如果后续需要区分“用户指定固定本机端口”和“始终自动分配”，再升级为显式 `portPolicy` 字段；本方案不在第一阶段引入该字段。

### 快照不变量

本地服务处于 `running` 时必须满足：

- `snapshot.config.port` 是实际绑定端口，范围为 `1..65535`；
- `service.boundAddress` 的端口与 `snapshot.config.port` 一致；
- `service.apiBaseUrl` 使用 loopback 主机和同一实际端口；
- `port = 0` 不得出现在 `running` 快照、持久化配置或前端可复制地址中。

本地服务处于 `starting` 且尚未绑定时，可以暂时保留候选配置的 `port = 0`，但不得生成伪 `apiBaseUrl`。

## 分组件实施步骤

### 1. shared

- 把 `ServerConfig.port` 的权威校验从无条件 `min = 1` 改为按模式判断：本机模式允许 `0..65535`，服务器模式要求 `1..65535`。
- 保持 `u16` 类型，不引入 nullable 端口，避免 JSON 和 Kotlin/TypeScript DTO 的结构性迁移。
- 不修改 `AppConfig::default()` 的全局 `17890`：该默认值仍服务于无头 server shell 和兼容配置；UI Shell 在首次 self-hosted 激活时自行把启动候选端口规范化为 `0`。
- 为 `port = 0` 增加配置语义注释和单元测试：self-hosted 允许，server-mode 拒绝。
- 确认 remote 模式的兼容行为不改变，不因隐藏 UI 字段而丢失已有配置。

### 2. core

- 保留现有 `TcpListener::bind(SocketAddr)`，确保 `0` 继续由操作系统分配。
- 在 `start_local_service` 或其平台适配边界提供“启动后读取实际地址”的稳定结果，不在 core 内持久化平台配置。
- 为动态端口增加测试：绑定成功返回非零实际端口、服务 HTTP 可访问、关闭后端口释放。
- 明确 core 不负责把实际端口写回 Android/桌面配置；回写属于 Shell 的配置事务。

### 3. Android native

- 调整 Android 请求校验：仅 `self-hosted` 接受 `port = 0`，`server-mode` 仍拒绝；保留 `1..65535` 的其它边界。
- `NativeServiceState` 已有 `bound_address`，补充稳定的实际端口解析或由上层统一从 `bound_address` 提取，避免重复绑定查询。
- 启动成功后返回“有效配置 + 实际服务状态”的结果，使 Kotlin manager 能构造端口已回写的 normalized config。
- 增加 Rust 测试覆盖 `port = 0` 的校验、实际绑定地址和错误映射。

### 4. Android Shell manager

把本地应用流程改为以下事务：

```text
读取候选配置
  -> self-hosted 且首次/需重新分配时使用 port=0
  -> native 校验
  -> 停止旧服务（如有）
  -> 启动 core 并取得 boundAddress
  -> 从 boundAddress 得到实际端口
  -> 构造 normalizedConfig(port=实际端口)
  -> 持久化 normalizedConfig
  -> 发布 running 快照
```

- 只有绑定成功后才持久化实际端口；启动失败不能把 `0` 或失败端口写入配置。
- `activateMissingDefault()` 在不改变 shared 全局默认配置的前提下，把首次 self-hosted 启动候选的端口改为 `0`；读取已有配置时不做这一转换。
- 保存失败时沿用当前恢复旧服务的逻辑，并保证恢复快照的 `config.port` 与恢复服务实际端口一致。
- 已保存端口冲突时，仅对 self-hosted 自动重试一次或使用端口 `0` 重新分配；仍失败则返回稳定错误和重试入口，避免无限重启循环。
- `server-mode` 端口冲突继续直接返回 `port_in_use`，不自动改端口。
- Android 进程重启后优先使用保存的实际端口，确保大多数情况下 API 地址不变。

### 5. Desktop Shell

桌面 Shell 尚未实现正式版本，实施时直接沿用 Android 的事务语义：

- 本机模式由 Shell 生成和持久化实际端口；
- 服务器模式使用用户固定端口；
- Bridge 返回的 `config.port`、`boundAddress` 和 `apiBaseUrl` 必须满足相同不变量；
- 端口分配不得由前端或 Tauri WebView 直接完成。

### 6. frontend

运行设置页面按模式呈现字段：

- `self-hosted`：隐藏端口字段和监听地址字段，只保留本机使用说明、服务状态和 Shell 返回的实际状态。
- `server-mode`：继续显示端口和监听地址，并保留局域网访问风险提示。
- remote：只显示远端服务地址和连接测试。

前端状态规则：

- `previewApiBaseUrl()` 对本机 `port = 0` 返回空值，不拼接 `http://127.0.0.1:0`。
- 本地 API 地址只读取 `runtimeSnapshot.service.apiBaseUrl`，不能根据草稿端口自行生成访问地址。
- 保存成功后使用返回快照的 normalized config 更新草稿，确保隐藏字段和实际端口同步。
- 本机模式切换时清理过时的端口字段错误；server-mode 切回本机不把服务器端口误显示为用户配置。
- 运行设置文档、页面代码地图和 Shell Bridge 文档同步说明端口所有权。

## 会话与地址变化

当前 refresh token 记录按 `api_base_url` 匹配，因此端口变化会导致旧 refresh token 不再恢复。第一阶段建议采用以下可控行为：

- 正常启动复用已持久化端口，保持会话连续性。
- 因端口冲突被迫换端口时，Shell 发布新地址；前端沿用现有运行地址变化流程清理当前会话并要求重新登录。
- 端口变化必须在状态区和 Notice 中明确反馈，不能让用户误以为凭据失效。

后续若需要端口变化后仍保持登录，再单独设计稳定的本地服务身份（例如 Shell 持久化安装级 `serviceId`，认证记录按服务身份而不是 URL 绑定）。不要在本次改动中简单删除 URL 绑定，否则远程服务切换可能扩大 refresh token 的适用范围。

## 兼容与迁移

- 已有 self-hosted 配置包含 `17890` 或其它有效端口：原样读取并优先复用，不强制重新分配。
- 已有 server-mode 配置：端口字段和冲突行为不变。
- 已有损坏配置：按现有 invalid 流程进入运行设置，不自动覆盖用户配置。
- 首次安装没有配置：使用 self-hosted `port = 0` 作为启动候选，成功后只持久化实际端口。
- Web fallback 无法真正管理本地 core：保留兼容默认端口和现有环境地址行为，不伪造动态端口；自动端口验收以 Android Shell 和未来 Desktop Shell 为准。
- 不修改数据库结构，不影响 core HTTP API 和业务数据文件。

## 验收与测试矩阵

以下是本功能的设计验收范围；本次实际执行的自动化命令和真机结果单独记录在“实际验收记录”中。

### shared/core/native

- self-hosted `port = 0` 校验通过并绑定成功；返回端口非零。
- server-mode `port = 0` 校验失败，字段为 `port`。
- 已占用固定端口返回 `port_in_use`，不创建新的数据库或文件副作用。
- 绑定失败后旧服务恢复，恢复快照端口和服务端口一致。
- 动态端口服务关闭后端口释放。

### Android manager

- 首次安装启动自动分配端口，配置文件保存实际端口而不是 `0`。
- 覆盖安装/进程重启优先复用同一端口，WebView 能恢复到同一 API 地址。
- 人为占用保存端口后启动自动换端口，快照和配置同步更新；必要时会话按既有规则重新登录。
- server-mode 端口冲突不自动切换，页面显示错误和重试。
- 旋转、后台恢复、前端刷新不重复启动或改变端口。

### frontend

- 390×844：本机模式不显示端口字段，状态正常且无横向溢出。
- 768px 附近：模式切换不出现混合断点或隐藏字段残留。
- server-mode 仍能编辑端口并展示局域网地址入口。
- 草稿 `port = 0` 不显示伪 API 地址；应用成功后显示 Shell 返回的真实地址。
- 端口变化、服务失败、重试和会话清理均有明确页面状态与 Notice。

### 真实设备

- Android API 33 ARM64 冷启动、二次覆盖安装、旋转、后台恢复和手势/三键导航。
- 真实设备上连续启动两次，确认端口和 API 地址稳定。
- 用临时进程占用保存端口，确认 self-hosted 自动换端口和 server-mode 固定端口错误路径。

## 分阶段交付

1. **契约与 core 基础（已完成）**：shared 条件校验、core/native 动态端口测试和实际地址返回。
2. **Android Shell 事务（已完成）**：绑定成功后回写实际端口、持久化、冲突重试和恢复逻辑。
3. **前端界面（已完成）**：按模式隐藏端口、删除 `port=0` 伪预览、同步文档和状态文案。
4. **Desktop 对齐（后续平台任务）**：正式 Desktop Shell 实现相同分配和快照不变量。
5. **设备验收（已完成本功能范围）**：首次动态分配、冲突换端口、进程重启复用端口和 loopback 健康检查。

## 实际验收记录

- Rust：`cargo test -p winestock-shared`、`cargo test -p winestock-android-native --lib` 通过。
- Core：`cargo test -p winestock-core local_service` 通过，动态端口、端口冲突和关闭释放路径共 4 项测试通过。
- Android JVM：`:app:testDebugUnitTest` 通过，覆盖首次自动分配、固定端口冲突回退、保存失败恢复、`server-mode` 不重试和运行地址不变量。
- 前端：`pnpm run build`、`pnpm run test:lan-access`、`pnpm run test:native-back` 通过。
- Android 构建：`:app:installDebug` 与 `:app:assembleRelease` 通过。
- 真机：API 33 ARM64 设备首次冲突后分配并持久化端口 `38141`，结束进程后再次启动仍复用该端口；WebView 请求 `http://127.0.0.1:38141/api/health` 并收到 `{"status":"OK"}`。
- 真机界面：Android MCP 确认 `self-hosted` 页面不显示端口或监听地址输入框，页面元素树和截图无异常。

## 实施门槛

- 每个阶段保持 `port = 0` 只存在于临时候选，不得写入 running 快照或持久化文件。
- 任何地址变化都必须通过 Shell Bridge 发布，不允许前端自行猜测。
- 变更 shared 公共校验、Shell Bridge DTO 或持久化语义时，同步更新根文档、Android 文档、前端运行设置文档、代码地图和测试。
- 完成前运行最窄有效 Rust/Android/frontend 检查，并至少完成一次真实 Android 设备启动与重启验证。
