# WineStock Android 引入 core 本地服务实施方案

> 文档状态：代码与 APK 集成已完成，最小真机 smoke 已通过，完整真机矩阵待测试<br>
> 涉及组件：`shared`、`core`、`server`、`android`、少量 `frontend` 运行状态联调<br>
> 编制日期：2026-07-23<br>
> 当前 Android 基线：AGP `9.2.1`、`minSdk 26`、`targetSdk 36`、Java 17、原生 WebView Shell

## 1. 结论

Android 引入 `core` 的正确方式不是让 Kotlin 直接调用 Rust 业务函数，也不是让 Axum 托管前端资源，而是：

1. 在 `core` 内补齐平台无关、可显式停止的本地服务运行句柄。
2. 新增独立的 `android/native` Rust `cdylib` 适配 crate，通过窄 JNI 接口管理 Tokio Runtime、配置校验和 core 生命周期。
3. Android 由 Application 级 `LocalCoreRuntimeManager` 持有本地服务状态，Activity 只使用该管理器，不拥有服务生命周期。
4. WebView 继续加载 Android 打包的 Vue 前端，并通过 `http://127.0.0.1:<port>` 使用完整 core HTTP API。
5. Shell Bridge 只传递运行配置、服务状态和启停命令，不代理库存、鉴权、文件等业务 API。
6. 第一阶段支持 `self-hosted`、`client-only` 和 `connect-to-remote`；`server-mode` 在 Foreground Service、通知和后台限制方案完成前继续禁用。
7. 当前阶段 Gradle 只使用 `cargo-ndk` 构建并验证 `arm64-v8a`，不支持 32 位或 x86 ABI，也不在普通 Android 构建中联网安装 Rust、target、NDK 或 cargo 工具。
8. 当前阶段只构建、校验和交付 APK，不接入 AAB 自定义任务或发布流程。

目标依赖关系如下：

```text
Android Kotlin Shell
  -> 具名 JNI 生命周期接口
  -> winestock-android-native（Android/Rust 适配层）
  -> winestock-core
  -> winestock-shared

Android WebView frontend
  -> http://127.0.0.1:<实际端口>
  -> winestock-core HTTP API
```

这里的关键边界是：**JNI 负责“启动哪个 core、core 当前是否运行”，HTTP 负责“如何使用 core 的业务能力”。**

## 2. 实施前基线与可行性结论

### 2.1 实施前 Android 只集成了前端

当前 Android Shell 已具备：

- Android variant 自动构建、校验和打包 Vue/Vite 前端资源。
- `WebViewAssetLoader` 与受信任 origin `https://winestock.internal`。
- AndroidX WebKit `WebMessageListener` Shell Bridge。
- 运行配置 DTO、SharedPreferences 持久化和运行快照。
- edge-to-edge、安全区发布、外部导航和应用恢复事件。

实施前本地 core 的缺口为：

- `RuntimeSnapshotFactory` 的 `startLocalService`、`stopLocalService`、`restartLocalService` 均为 `false`。
- 本地模式由 `ShellBridgeHost` 返回 `unsupported_runtime_mode`。
- `RuntimeConfigValidator.kt` 是 `winestock_shared` 规则的 Kotlin 镜像，不是 Rust 权威校验。
- `ShellBridgeHost` 当前同步分发请求，不适合在 WebMessage/UI 线程执行数据库打开、migration 或 JNI 启停。
- Activity 当前只拥有 WebView，不存在应用进程级 core 管理器。

### 2.2 core 当前已具备的基础

`core` 已提供：

- `bootstrap_from_config()`：打开 SQLite、执行 migration、补齐 RBAC 和默认库存模板、清理临时图片、初始化鉴权。
- `bind_server()`：按 `ServerConfig` 绑定 TCP listener，并区分非法地址、端口占用和运行错误。
- `BoundServer::serve_local_with_shutdown()`：使用本地服务状态启动 Axum，并响应平台提供的 graceful shutdown future。

当前缺口不是业务能力，也不是 Android 不兼容，而是缺少一个供平台 Shell 长期持有的统一运行句柄。现有 `server` shell 仍自行拼装：

```text
bootstrap_from_config
  -> bind_server
  -> serve_local_with_shutdown
```

Android 若照抄这段编排，会产生第二套启动、停止、错误映射和异常退出处理，因此必须先在 `core` 收敛公共生命周期 API。

### 2.3 已执行的交叉编译验证

本次分析已执行：

```text
cargo ndk -t arm64-v8a -P 26 check -p winestock-core --locked
```

结果通过。它证明当前 `winestock-core` 及其主要依赖链可以交叉编译到 Android ARM64，包括：

- Axum / Tokio 网络运行时；
- SeaORM / SQLx / SQLite；
- migration；
- JWT、Argon2 和随机数依赖；
- OpenAPI / Swagger UI 相关依赖。

验证时本机环境为：

| 项目                 | 已验证值        |
| -------------------- | --------------- |
| Rust                 | `1.96.1`        |
| cargo-ndk            | `4.1.2`         |
| Android NDK          | `30.0.14904198` |
| Android Rust targets | 已安装          |
| Android 最低 API     | `26`            |

该检查只证明 core 依赖可交叉编译，不等于 JNI、Gradle 打包、设备加载和 WebView HTTP 联调已经完成。后续仍需完成 ARM64 JNI 构建、APK 包级验证和真实设备 smoke。

## 3. 目标与非目标

### 3.1 目标

- Android 默认 `self-hosted` 模式可在应用进程内启动同一套 core/Axum 服务。
- WebView 使用真实 loopback API 地址，不使用 `0.0.0.0` 或其它绑定语义地址。
- core 启动、停止、异常退出和端口占用有统一、稳定的 Shell 错误码。
- Activity 旋转、重建和短暂前后台切换不会反复关闭并重启数据库与 Axum。
- native library 缺失或加载失败时，前端资源和运行设置页仍能正常打开。
- Android 配置应用具备“激活成功后提交”和失败回滚语义。
- Android 不再长期维护一套与 `winestock_shared` 平行的配置校验规则。
- Debug/Release APK 都能证明只包含目标 ABI 的正确 `.so`。
- `server` shell 改用同一 core 运行句柄，证明公共生命周期 API 不是 Android 专用包装。

### 3.2 非目标

- 不通过 JNI 暴露鉴权、库存、文件、用户或其它业务方法。
- 不让 Axum 服务 Android 前端资源。
- 不把 Android 改造成 Rust `NativeActivity`，也不替换当前 Kotlin/WebView Shell。
- 不在第一阶段支持后台长期对其它设备提供服务的 `server-mode`。
- 不在第一阶段增加原生设置 Activity、通知中心或第二套错误 UI。
- 不顺带更换 SQLite、SeaORM、Axum、前端框架或 Shell Bridge v1。
- 不在 Gradle 普通构建中自动安装 Rust、rustup target、cargo-ndk 或 NDK。
- 不为了减小包体立即裁剪 Swagger/OpenAPI 功能；先完成一致性集成并测量，再依据实际包体决定是否 feature-gate。

## 4. 组件职责与最终结构

### 4.1 `shared`

负责：

- `AppConfig`、`ServerConfig`、`StorageConfig`、`RuntimeMode`。
- 平台无关默认值和权威字段校验。
- 必要时提供结构化的配置校验问题，供平台适配层映射字段。

不负责：

- Android 路径选择、SharedPreferences、JNI、Activity 或服务启停。

### 4.2 `core`

负责：

- bootstrap、数据库、migration、业务状态和 HTTP Router。
- TCP 绑定、Axum 运行、graceful shutdown。
- 平台无关的 `RunningLocalService` 句柄和运行错误。

不负责：

- 创建 Android 目录、加载 `.so`、监听 Activity 或决定 Foreground Service。

### 4.3 `android/native`

负责：

- JNI 字符串和 JSON 边界。
- 长期持有 Tokio Runtime。
- 把 Android DTO 与固定平台路径转换成 `AppConfig`。
- 调用 core 运行句柄并把 Rust 错误映射成稳定 native 协议。
- 捕获 panic，保证 unwind 不越过 FFI。

不负责：

- WebView、SharedPreferences、Activity、通知或业务 HTTP DTO。

### 4.4 Android Kotlin Shell

负责：

- 安全加载 native library。
- 决定 app-private 存储路径并预创建目录。
- Application 级 core 生命周期、配置事务和快照发布。
- Shell Bridge 异步调度、主线程回复和页面 generation 防护。
- 前后台策略和未来 Foreground Service。

### 4.5 frontend

负责：

- 运行设置和服务错误 UI。
- 根据快照选择 `apiBaseUrl`。
- 地址切换时取消旧请求、清理旧服务会话并重新初始化 API client。
- 通过 HTTP 使用全部业务能力。

## 5. Rust crate 结构设计

### 5.1 新增 Android 适配 crate

建议新增 workspace member：

```text
android/native/
  Cargo.toml
  src/
    lib.rs
    engine.rs
    contract.rs
    config.rs
    error.rs
    ffi.rs
```

包名建议：

```toml
[package]
name = "winestock-android-native"

[lib]
name = "winestock_android_native"
crate-type = ["cdylib", "rlib"]
```

其中：

- `cdylib` 产出 Android 加载的 `libwinestock_android_native.so`。
- `rlib` 仅用于宿主机单元/集成测试；若最终测试结构不需要，可只保留 `cdylib`。
- `engine.rs`、`contract.rs` 和配置映射尽量保持宿主机可测试。
- 只有 `ffi.rs` 和 Android 日志初始化使用 `cfg(target_os = "android")`。

根 workspace 增加成员：

```text
members = [
  "android/native",
  "core",
  "server",
  "shared",
]
```

### 5.2 为什么使用 jni-rs，而不是把 JNI 写进 core

推荐使用当前核对的 `jni 0.22.4`：

- 当前跨语言表面很小，只需要字符串、JSON 和少量生命周期函数。
- 不需要 UniFFI 的生成代码、额外 Kotlin 运行时、JNA 包装和 bindings 同步流程。
- 具名 JNI 方法更容易限制能力并做包级验证。
- `core` 与 `shared` 可以继续保持 `#![forbid(unsafe_code)]` 和平台无关。

JNI/FFI 是 Android 平台边界，必须只存在于 `winestock-android-native`。该 crate 建议使用：

```rust
#![deny(unsafe_op_in_unsafe_fn)]
```

如果确实需要极少量 `unsafe`，每处都必须有中文 `SAFETY` 注释、独立测试和代码审查；不得把 `unsafe` 扩散到 core/shared。

### 5.3 建议依赖

`android/native` 第一阶段只引入：

- `winestock-core`；
- `winestock-shared`；
- `jni = 0.22.4`；
- `serde` / `serde_json`；
- workspace `tokio`；
- `log` 与 Android target 下的 `android_logger = 0.15.1`，仅用于 native 边界日志。

不要引入：

- JNI 回调框架；
- 通用 RPC/反射桥；
- UniFFI；
- Android NDK CMake/C++ 层；
- 第二套 HTTP client 或业务 DTO。

依赖落地前仍需按项目依赖检查规则再次核对 crates.io 当前稳定版本并更新 `Cargo.lock`。

## 6. core 公共运行句柄

### 6.1 目标 API

建议在 `core/src/local_service.rs` 收敛以下候选 API：

```rust
pub struct LocalServiceInfo {
    pub bound_addr: SocketAddr,
    pub database_path: PathBuf,
    pub files_dir: PathBuf,
    pub admin_setup_required: bool,
}

pub struct RunningLocalService {
    // 实际字段保持私有：shutdown sender、serve task 和启动信息。
}

impl RunningLocalService {
    pub fn info(&self) -> &LocalServiceInfo;
    pub fn is_finished(&self) -> bool;
    pub async fn shutdown(self) -> Result<(), LocalServiceRuntimeError>;
    pub async fn wait(self) -> Result<(), LocalServiceRuntimeError>;
}

pub async fn start_local_service(
    config: &AppConfig,
) -> Result<RunningLocalService, LocalServiceRuntimeError>;
```

最终签名可以随实现调整，但必须保证：

- 句柄是非 `Clone` 的唯一所有权对象。
- 启动成功时端口已经真实绑定。
- 可读取操作系统返回的实际地址。
- 显式 shutdown 会等待 Axum graceful shutdown 和任务结束。
- 可识别服务是否意外退出，并取得稳定错误。
- 重复停止的行为明确，不能 panic。
- 句柄被意外 drop 时至少释放 shutdown sender；正常路径仍必须显式等待关闭。

### 6.2 启动顺序

推荐 `start_local_service()` 使用以下顺序：

```text
确认 mode 需要本地服务
  -> bind_server（先失败于非法地址或端口占用）
  -> bootstrap_from_config（打开数据库、migration、业务初始化）
  -> 构造 Router
  -> 建立 shutdown channel
  -> tokio::spawn Axum serve task
  -> 返回 RunningLocalService
```

先绑定端口的原因：

- 配置应用时可以在数据库 migration 前发现端口冲突。
- 端口冲突不会产生不必要的存储初始化副作用。
- listener 在后续 bootstrap 失败时随局部对象 drop 自动释放。

bootstrap 期间虽然端口已绑定，但 Shell 状态仍是 `starting`，前端不能在收到 `running` 快照前把该地址视为可用。

### 6.3 运行错误

新增 `LocalServiceRuntimeError`，至少区分：

- 当前模式不支持本地服务；
- core bootstrap 失败；
- bind 失败；
- serve 失败；
- Tokio 任务 join 失败；
- graceful shutdown 失败。

该错误保留 source chain，Android adapter 再按具体 variant 映射稳定 Shell 错误码，不能让 Kotlin 解析英文 `Display` 文案。

### 6.4 graceful shutdown 与超时

core 提供可等待的正常 shutdown，不在库内硬编码 Android 或 server 的超时策略。

- `server` 可以收到 Ctrl+C 后直接等待关闭。
- Android native adapter 建议给配置切换和显式停止设置有限超时，例如 5 秒。
- 超时后记录 `service_crashed` 或 `service_start_failed` 的安全错误，并中止残留 task，不能无限占用串行 executor。
- Tokio workspace features 需要补充 `sync`；若适配层使用 `tokio::time::timeout`，同时补充 `time`。

### 6.5 让 server 复用同一 API

`server/src/lib.rs` 应从手工编排改为：

```text
读取并校验配置
  -> 准备平台目录
  -> start_local_service
  -> 读取 LocalServiceInfo 输出地址与存储状态
  -> 等待 Ctrl+C
  -> RunningLocalService::shutdown
```

这样可以验证：

- Android 与 server 使用完全相同的 bootstrap/bind/serve 入口。
- 端口占用、serve 异常和 shutdown 语义只有一份实现。
- `core` API 确实是平台无关能力，不是仅为 JNI 增加的特殊路径。

## 7. Android native engine 与 JNI 契约

### 7.1 进程内单例

推荐 native 层使用：

```rust
static ENGINE: OnceLock<Mutex<Option<NativeEngine>>> = OnceLock::new();

struct NativeEngine {
    runtime: tokio::runtime::Runtime,
    service: Option<RunningLocalService>,
    active_config_fingerprint: Option<String>,
    last_exit: Option<NativeRuntimeError>,
}
```

不推荐用 `jlong` 保存裸指针：

- Application 级运行时本来就是进程唯一实例。
- 避免 Kotlin 丢失 handle、重复释放或跨 Activity 生命周期传递指针。
- `Mutex<Option<...>>` 允许测试时显式销毁和重建 engine。

### 7.2 Tokio Runtime

`NativeEngine` 在首次初始化时创建一个长期 multi-thread Tokio Runtime：

```rust
tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .thread_name("winestock-core")
    .build()
```

实施要求：

- JNI 只能由 Kotlin 专用后台 executor 调用。
- Rust 在这个非 Tokio、非 UI 线程上执行 `runtime.block_on()`。
- 不在 Tokio worker 内创建嵌套 Runtime，也不直接嵌套 `block_on()`。
- Runtime 生命周期长于每次服务启动；重启 Axum 不重建线程池。
- worker 数量在设备性能测试后确定；初期可显式使用较小固定值，避免按大核数量创建过多常驻线程。
- engine 销毁必须先停止本地服务，再在非 async/JNI worker 上关闭 Runtime。

Tokio 当前文档明确：`Runtime::block_on` 会阻塞调用线程，且在 async context 内调用会 panic。因此 Kotlin executor 隔离是正确性要求，不只是性能优化。

### 7.3 具名 JNI 方法

建议 Kotlin 暴露内部对象：

```kotlin
@Keep
internal object NativeCoreBridge {
    external fun nativeInitialize(requestJson: String): String
    external fun nativeDefaultRuntimeConfig(): String
    external fun nativeValidateRuntimeConfig(requestJson: String): String
    external fun nativeStartLocalService(requestJson: String): String
    external fun nativeStopLocalService(): String
    external fun nativeRestartLocalService(requestJson: String): String
    external fun nativeGetRuntimeState(): String
    external fun nativeShutdownEngine(): String
}
```

约束：

- 所有复杂输入和输出都是 UTF-8 JSON 字符串。
- 不传递 Rust 对象、数据库连接、Java callback 或业务 DTO。
- 每个方法只有一个明确职责。
- `@Keep` 防止未来开启 R8 后类名或 native method 被改写。
- `System.loadLibrary()` 使用名称 `winestock_android_native`，不包含 `lib` 和 `.so`。

### 7.4 native 协议版本

JNI JSON 应有独立于 Shell Bridge 的协议版本：

```json
{
  "nativeProtocolVersion": 1,
  "ok": true,
  "result": {}
}
```

失败示例：

```json
{
  "nativeProtocolVersion": 1,
  "ok": false,
  "error": {
    "code": "port_in_use",
    "message": "本地服务端口已被占用",
    "field": "port"
  }
}
```

这样可以在 Kotlin 与 `.so` 来自不同构建时明确报告版本不匹配，而不是错误解析字段。

### 7.5 panic 与 JNI 异常边界

每个导出函数必须：

1. 校验 `JString` 是否存在且可转换。
2. 校验 native 协议版本和 JSON 结构。
3. 使用 `catch_unwind` 包裹 Rust 入口。
4. 把 panic 映射为安全、稳定的失败响应。
5. 把完整错误链只记录到 Logcat，不把绝对路径、SQL 或内部堆栈返回前端。
6. 禁止 panic、Rust reference 或未处理 JNI exception 越过 FFI。

native adapter 不从 Tokio worker 主动回调 JVM，避免线程附着、局部引用生命周期和 Activity 泄漏问题。Kotlin 通过命令结果、主动状态查询和前端 HTTP 健康检查发现状态变化。

## 8. Android Application 级生命周期

### 8.1 新增 Application

新增：

```text
android/app/src/main/java/winestock/xiaowine/cc/WineStockApplication.kt
```

并在 Manifest 设置：

```xml
<application android:name=".WineStockApplication" ... />
```

`WineStockApplication.onCreate()` 只做轻量、非阻塞初始化：

```text
创建 LocalCoreRuntimeManager
  -> 后台加载 native library
  -> 读取或创建默认运行配置
  -> 为本地模式准备存储目录并异步启动 core
```

不能在主线程等待：

- Rust Runtime 创建；
- SQLite 打开；
- migration；
- core bootstrap；
- TCP 绑定。

WebView 和前端资源应与 core 启动并行，前端先看到 `starting`、`failed` 或 `running` 快照。

### 8.2 LocalCoreRuntimeManager

建议新增：

```text
android/app/src/main/java/winestock/xiaowine/cc/core/
  LocalCoreRuntimeManager.kt
  NativeCoreBridge.kt
  NativeLibraryLoader.kt
  NativeContract.kt
  AndroidStoragePaths.kt
```

`LocalCoreRuntimeManager`：

- 使用 `applicationContext`，不得持有 Activity/WebView。
- 持有一个单线程 `ExecutorService`，串行化配置、start、stop、restart 和状态查询。
- 持有不可变 `RuntimeSnapshot` 状态，并在变更时通知订阅者。
- 保证同一时刻最多存在一个本地 core。
- 对 `start`、`stop`、`restart` 实现幂等或明确错误。
- 在显式配置切换和停止时执行 graceful shutdown。

### 8.3 状态机

建议内部状态机：

```text
Uninitialized
  -> LoadingNative
  -> Stopped / RemoteConfigured
  -> Starting
  -> Running
  -> Stopping
  -> Stopped

任意初始化或运行步骤失败
  -> Failed(error)
```

关键规则：

- `Running -> start`：同一生效配置直接返回当前快照。
- `Starting -> start/restart`：串行排队，不并发启动第二个实例。
- `Stopped -> stop`：幂等返回 stopped。
- `Failed -> restart`：清除本次运行错误后重新执行完整启动。
- 服务 task 意外结束：下一次状态查询立即转为 `failed/service_crashed`。
- 本地配置下显式 stop 只影响当前进程；下一次冷启动仍按固定 auto-start 策略启动。

### 8.4 Activity 生命周期

`MainActivity` 调整为：

- 从 `WineStockApplication` 获取同一个 manager。
- 安装 `ShellBridgeHost` 时传入 manager。
- `onResume()` 请求 manager 刷新 native 状态，再通知前端应用恢复。
- `onDestroy()` 只解除 Shell Bridge/事件订阅和 WebView 相关对象，不停止 core。

因此以下情况不会重启服务：

- 屏幕旋转；
- Activity 因配置变化重建；
- 短暂进入系统选择器或浏览器；
- WebView 页面重载。

不要依赖 `Application.onTerminate()` 做真实设备关闭；Android 正常设备通常不会调用它。进程被系统杀死时无法保证 graceful shutdown，下一次启动应依靠 SQLite WAL 恢复并重新启动 core。

### 8.5 后台与 server-mode

第一阶段 `self-hosted` 的语义是“供当前应用 UI 使用”：

- Activity 暂时不可见时不立即停止。
- 进程被系统回收时服务随进程结束。
- 不承诺在后台长期为其它设备提供服务。

`server-mode` 需要单独实现：

- Android `Service`；
- 常驻通知和通知渠道；
- `FOREGROUND_SERVICE` 相关权限及适用 service type；
- `startForegroundService()` 后及时调用 `startForeground()`；
- Android 12+ 禁止从后台任意启动 Foreground Service 的限制；
- Android 13+ 通知权限体验；
- task removed、系统重启和用户显式停止策略。

在这些内容完成前：

```text
capabilities.serverMode = false
```

并由 Android 平台策略拒绝 `server-mode`，不能仅因为 shared 枚举支持它就开放。

## 9. native library 安全加载

不要在无保护的静态初始化器中直接调用 `System.loadLibrary()`，否则缺少 ABI 或 `.so` 损坏会在应用类加载阶段导致不可恢复崩溃。

建议 `NativeLibraryLoader`：

```text
未尝试
  -> loading
  -> loaded
  -> failed(UnsatisfiedLinkError / SecurityException)
```

加载失败时：

- 捕获 `UnsatisfiedLinkError` 和相关异常。
- manager 进入 `failed/native_library_unavailable`。
- 本地服务 capabilities 设为 `false`。
- WebView、前端和运行设置仍正常加载。
- 已保存的远端配置仍可继续使用，前端也可以通过受限的 Kotlin 降级校验切换到远端模式。
- Logcat 记录 ABI、应用版本和安全错误链，但不向页面泄漏本机路径。

为了避免某个 ABI 的 `.so` 打包故障同时破坏原有远端客户端能力，native 不可用时允许一个明确受限的降级路径：

- 只接受 `client-only` 和 `connect-to-remote`。
- Kotlin 只校验 http/https、合法 host、无凭据/查询/hash，且拒绝 `0.0.0.0`、`::`。
- `self-hosted` 和 `server-mode` 一律返回 `native_library_unavailable` 或 `unsupported_runtime_mode`。
- 该降级校验必须独立命名、单独测试，不能重新扩展成 shared 全量规则镜像。

## 10. 配置与存储

### 10.1 Editable DTO 与 AppConfig 映射

前端继续提交：

```text
EditableRuntimeConfig
  mode
  bindHost
  port
  remoteBaseUrl
```

正常路径下 Kotlin 只负责结构解析和持久化，不再长期复制 shared 业务校验。native adapter 负责映射：

```text
mode              -> AppConfig.server.mode
bindHost          -> AppConfig.server.bind_host
port              -> AppConfig.server.port
remoteBaseUrl     -> AppConfig.server.remote_base_url
local mode        -> auto_start_server = true
Android fixed path -> AppConfig.storage.database_path/files_dir
Android policy     -> auto_migrate = true
```

建议在 `shared` 增加结构化校验问题读取 API，使 adapter 能把：

```text
server.bind_host      -> bindHost
server.port           -> port
server.remote_base_url -> remoteBaseUrl
server.mode           -> mode
```

映射为 Shell Bridge v1 的 `fieldErrors`，禁止解析 `garde::Report` 的显示文案。

### 10.2 Android 平台附加策略

shared 负责通用合法性，Android adapter 还需执行平台策略：

- `self-hosted` 第一阶段只允许 loopback 地址，推荐仅允许 `127.0.0.1`；如要支持 IPv6，再单独验证 WebView 的 `[::1]` 地址格式。
- `server-mode` 返回 `unsupported_runtime_mode`。
- 远端模式要求合法 http/https URL，但不要求应用配置时服务在线。
- `0.0.0.0`、`::` 只能作为监听语义，绝不能返回为 `apiBaseUrl`。

禁止 Android 在 `self-hosted` 下静默监听所有网卡，否则应用进入后台后会形成没有 Foreground Service 保障的局域网服务器。

### 10.3 存储目录

建议使用 app-private 且不进入 Android Auto Backup 的目录：

```text
context.noBackupFilesDir/winestock/
  data/
    winestock.sqlite
    winestock.sqlite-wal
    winestock.sqlite-shm
    files/
```

选择 `noBackupFilesDir` 的原因：

- SQLite 数据库与 `files/` 大对象仓必须保持一致。
- 当前 backup XML 仍是模板，直接自动备份可能只恢复部分数据。
- SQLite WAL/SHM 和文件目录不适合被无事务地分开恢复。
- app-private 目录不需要外部存储权限。

目录职责：

- Kotlin `AndroidStoragePaths` 解析绝对路径。
- Kotlin 在调用 native/core 前创建数据库父目录和 `files/`。
- core 继续拒绝缺失的数据库父目录，不接管平台目录创建。
- 前端快照和错误不得返回绝对路径。

未来若需要设备迁移，应设计显式的导出/导入或一致性备份，不应直接把当前数据库和文件目录加入 Auto Backup。

### 10.4 RuntimeConfigStore

第一阶段可继续使用当前版本化 SharedPreferences 保存 `EditableRuntimeConfig`，原因是：

- 它只保存模式、地址和端口，不保存数据库路径、token 或业务数据。
- 平台路径每次由当前设备重新派生，避免恢复旧设备绝对路径。
- 改动面小，便于聚焦 core 生命周期。

需要调整：

- 正常默认值由 native/shared 提供，Kotlin 常量只保留 native 不可用时的界面降级值。
- 删除 `RuntimeConfigValidator.kt` 的全量规则镜像；如需保证 native 缺失时仍能连接远端，只保留独立、受限的远端 URL 降级校验器。
- 结构解析失败仍保留可修复草稿，但权威校验必须走 native/shared。
- 如果当前版本已发布，读取旧 `config.v1` 后成功校验即迁移到新版本 key，再删除旧 key。
- 如果尚未发布且无需保留用户配置，直接替换旧实现，不维护双存储层。

### 10.5 首次启动

配置缺失时：

```text
native/shared 默认配置
  -> 映射为 Android EditableRuntimeConfig
  -> 保存为当前正式配置
  -> createdDefault = true
  -> 准备 app-private 目录
  -> 启动 self-hosted core
```

若保存默认配置失败，不应假装 configured；前端进入可修复错误态。若 core 启动失败，默认配置仍可保留，快照为 `failed`，用户可以修改端口或切换远端模式。

## 11. 配置应用事务与回滚

### 11.1 本地配置

本地配置必须执行“激活成功后提交”：

```text
收到候选配置
  -> native/shared 权威校验
  -> 保存旧配置、旧服务状态
  -> 发布 starting
  -> 停止需要替换的旧本地服务
  -> 使用候选配置启动 core
  -> 启动成功后保存候选配置
  -> 发布 running + 实际地址
```

任一步骤失败时：

```text
停止候选服务（若已启动）
  -> 恢复旧正式配置
  -> 尽力恢复旧本地服务或旧远端快照
  -> 发布失败快照
  -> applied = false
```

特别注意：

- 若候选 core 已启动但 SharedPreferences 提交失败，必须关闭候选并恢复旧状态。
- 数据库 migration 可能产生不可逆的前向变化；当前 Android 不允许用户编辑存储位置，候选和旧服务使用同一数据库，因此回滚只恢复运行配置和服务，不回滚 schema。
- 如果旧服务恢复也失败，返回候选错误为主错误，并在日志记录 rollback error，快照进入 `failed`。

### 11.2 远端配置

远端模式：

```text
权威校验 URL
  -> 停止当前本地服务
  -> 保存远端配置
  -> 发布 ownership=remote 快照
  -> 前端自行执行 HTTP 健康检查
```

远端暂时不可访问不能阻止保存，因为服务可能离线。保存失败时应尽力恢复原本地服务。

### 11.3 地址变化后的前端边界

当 `apiBaseUrl` 变化时，前端必须继续执行：

1. 停止旧健康检查与自动刷新。
2. 取消旧 API 地址上的进行中请求。
3. 清理内存 access token 和旧服务会话。
4. 配置新的 `apiBaseUrl`。
5. 对新地址重新执行 `/api/health`。
6. 重新初始化鉴权会话。

不得把旧服务 refresh/access token 发送到新 core 实例或远端地址。

## 12. Shell Bridge 异步改造

### 12.1 当前问题

当前 `ShellBridgeHost.handleMessage()` 同步执行 `dispatch()` 并立即回复。core 接入后，以下操作可能耗时：

- native library 初始化；
- 配置权威校验；
- SQLite 打开和 PRAGMA；
- migration 与业务 bootstrap；
- TCP 绑定；
- graceful shutdown。

这些操作不能运行在 WebMessage/UI 回调线程。

### 12.2 目标流程

```text
WebMessageListener 收到请求（主线程）
  -> 解析并做最小结构校验
  -> 捕获 request id、reply proxy、page generation
  -> 提交 LocalCoreRuntimeManager 单线程 executor
  -> manager/native 执行命令
  -> Handler(Looper.getMainLooper()) 回主线程
  -> generation 仍有效时通过 JavaScriptReplyProxy 回复
```

`frontendReady`、受控 `openExternal` 等纯 UI 操作可以在主线程执行，但生命周期命令必须统一走异步路径。

### 12.3 页面 generation

WebView 重载或导航后，旧请求可能晚到。`ShellBridgeHost` 应维护单调递增 generation：

- 新主文档建立消息代理时增加 generation。
- 每个异步请求记录发起 generation。
- 回复时只向仍然匹配的 generation/proxy 发送。
- 迟到结果仍可更新 manager 的进程状态，但不得回复到新页面的无关 request id。

Activity 销毁时解除 manager 事件订阅，避免旧 WebView 泄漏。

### 12.4 capabilities

native library 加载且 engine 初始化成功时：

```text
startLocalService = true
stopLocalService = true
restartLocalService = true
serverMode = false
```

native 不可用时前三项为 `false`；远端模式仍可走受限降级路径，`applyRuntimeConfig(self-hosted)` 则返回 `native_library_unavailable`。如果 native 可用但当前快照是远端模式，生命周期 capability 可以按前端交互需要关闭，切回本地统一通过 `applyRuntimeConfig(self-hosted)` 激活。

### 12.5 生命周期命令语义

- `startLocalService`：只使用当前已提交的本地配置；远端模式返回 `unsupported_runtime_mode`。
- `stopLocalService`：幂等停止当前本地服务，不修改已提交模式。
- `restartLocalService`：使用当前已提交本地配置执行 stop + start。
- `applyRuntimeConfig`：唯一可以切换正式配置的命令。
- 所有阶段变化都发布 `runtimeStateChanged`。

## 13. 网络与安全约束

### 13.1 页面地址与 API 地址

必须保持：

```text
WebView 页面地址  https://winestock.internal/
本地 API 地址    http://127.0.0.1:<实际端口>
```

Axum 不服务 `frontend/dist`，Android 前端在 core 完全不可用时仍可打开设置页。

### 13.2 loopback 与端口

- `self-hosted` 第一阶段强制 loopback。
- 默认端口继续使用 `17890`。
- 如果支持配置端口 `0` 作为自动分配，需要先修改 shared 契约和前端校验；第一阶段不引入。
- 端口占用必须映射为 `port_in_use`，并关联 `port` 字段。
- `boundAddress` 返回真实监听地址。
- `apiBaseUrl` 始终使用可访问 loopback 地址，不根据 `0.0.0.0` 拼接。

### 13.3 WebView cleartext、mixed content 与 CORS

当前 Android 已允许：

- cleartext HTTP；
- `https://winestock.internal` 到 HTTP API 的 mixed content；
- core 全局 CORS/OPTIONS。

接入本地 core 时必须通过真实 WebView 验证：

- `/api/health`；
- 登录/刷新预检；
- multipart 图片上传；
- 文件下载；
- Android WebView 是否触发 Private Network Access 额外预检。

如果设备实际出现 PNA 拒绝，再依据请求头补充 core CORS 响应；不要在未观察到需求时引入平台特定 header。

### 13.4 信任边界

- 非受信任 WebView origin 不能调用 Shell Bridge 启停服务。
- JNI 方法是应用内部实现，不暴露给前端任意 invoke。
- business API 继续依赖 core 鉴权和权限模型。
- 前端错误不得包含数据库绝对路径、SQL、Rust backtrace 或 JNI 内部信息。
- `self-hosted` 禁止监听局域网，降低本地进程暴露面。
- 实施时需复核 core 所有未鉴权 HTTP 路径，确认本地应用场景不会引入首次管理员或敏感初始化竞态。

## 14. Gradle、cargo-ndk 与 ABI 打包

### 14.1 支持 ABI

当前阶段只支持：

| ABI         | 用途                      |
| ----------- | ------------------------- |
| `arm64-v8a` | 当前目标 Android 真实设备 |

当前阶段明确不加入：

- `armeabi-v7a` 等 32 位 ARM ABI；
- `x86`；
- `x86_64`。

以后只有出现明确设备或 CI 需求，并单独完成依赖兼容、包体和运行验证后，才另行扩展 ABI；本方案当前不为这些 ABI 保留构建分支。

当前制品格式明确为：

- Debug APK；
- Release APK（未配置发布签名时为 unsigned APK）。

当前不构建、不校验、不发布 AAB；标准 AGP bundle 能力不属于本项目当前验收和发布流程。

### 14.2 Android 配置

`android/app/build.gradle.kts` 建议固定：

```kotlin
android {
    ndkVersion = "30.0.14904198"

    defaultConfig {
        ndk {
            abiFilters += "arm64-v8a"
        }
    }
}
```

NDK 版本应使用本次已经验证且 CI 可安装的版本；升级必须重新执行 ARM64 交叉编译和设备验证。

### 14.3 构建命令

Debug：

```text
cargo ndk \
  -t arm64-v8a \
  -P 26 \
  -o <debug-generated-jniLibs> \
  build -p winestock-android-native --locked --offline
```

Release：

```text
cargo ndk \
  -t arm64-v8a \
  -P 26 \
  -o <release-generated-jniLibs> \
  build -p winestock-android-native --release --locked --offline
```

输出：

```text
<generated-jniLibs>/
  arm64-v8a/libwinestock_android_native.so
```

### 14.4 Gradle 任务图

复用当前 `android/buildSrc` 的前端构建模式，新增 Rust 任务：

```text
assemble<Variant> / install<Variant>
  -> package/merge native libs
     -> verify<Variant>RustNativeLibraries
        -> build<Variant>RustNativeLibraries
```

已新增：

```text
android/buildSrc/src/main/kotlin/winestock/build/
  RustNativePackagingTasks.kt
    - RustNativeBuildTask
    - RustNativeVerifyTask
    - RustNativeApkVerifyTask
```

任务要求：

- 输入包含根 `Cargo.toml`、`Cargo.lock`、`android/native/**`、`core/**`、`shared/**` 和相关构建配置。
- 排除 `target/`、Android `build/`、日志和其它生成目录。
- Debug/Release 使用不同 `CARGO_TARGET_DIR` 和 generated jniLibs 输出，避免并发污染。
- 通过 AGP variant generated `jniLibs` 接入，不写入 `app/src/main/jniLibs`。
- 普通构建不调用 `rustup target add`、`cargo install`、SDK manager 或网络安装。
- `--offline` 缺依赖时立即失败，并提示先执行显式环境准备。
- 初期不启用共享远程构建缓存；先证明跨机器产物和符号处理稳定。

### 14.5 环境准备

联网或已配置代理的显式准备阶段：

```text
rustup target add aarch64-linux-android
cargo install cargo-ndk --version 4.1.2 --locked
cargo fetch --locked
```

Android/Google Maven 依赖、Gradle distribution 和 NDK 同样应在可联网阶段准备。普通 `assemble` 不应因为工具缺失而静默联网或长时间等待。

### 14.6 `.so` 验证

`verify<Variant>RustNativeLibraries` 至少检查：

1. `arm64-v8a` 目录和目标 `.so` 存在、非空，且没有意外生成或打包其它 ABI。
2. ELF machine 与目录 ABI 一致。
3. `.so` 导出预期 JNI symbols。
4. `DT_NEEDED` 不包含未打包的意外动态库。
5. Debug/Release 没有互相复用错误产物。
6. 输出中不包含本机绝对源码路径的可见泄漏。

最终 APK 再检查：

```text
APK:
  lib/arm64-v8a/libwinestock_android_native.so
```

Release 流水线还应保存与 APK 精确对应的 native symbols，供本地 tombstone 和崩溃日志解析使用。

## 15. 稳定错误映射

Android native adapter 必须按 Rust error variant 映射，不得解析 `Display` 文案。

| 稳定错误码                   | 主要来源                                                | 前端建议行为                |
| ---------------------------- | ------------------------------------------------------- | --------------------------- |
| `native_library_unavailable` | `.so` 缺失、ABI 不匹配、load/init 失败                  | 保持设置页可用，允许切远端  |
| `config_unavailable`         | SharedPreferences 读写失败、默认配置无法提交            | 提示重试或检查设备存储      |
| `config_invalid`             | shared 权威校验失败                                     | 映射到字段错误              |
| `storage_unavailable`        | app-private 目录创建失败、目录状态异常                  | 展示存储错误，不暴露路径    |
| `database_open_failed`       | `StorageBootstrapError::OpenDatabase/ConfigureDatabase` | 提示数据库无法打开          |
| `migration_failed`           | `StorageBootstrapError::MigrateDatabase`                | 提示升级失败并保留设置入口  |
| `invalid_bind_host`          | `ServerStartError::InvalidBindHost` 或 Android 策略拒绝 | 关联 `bindHost`             |
| `port_in_use`                | bind 的 `AddrInUse`                                     | 关联 `port`，允许修改后重试 |
| `service_start_failed`       | 其它 bootstrap/bind/start 失败                          | 展示通用启动失败并允许重试  |
| `service_crashed`            | Axum serve task 意外退出、join 失败                     | 清除运行地址并允许 restart  |
| `unsupported_runtime_mode`   | Android `server-mode` 或远端配置调用本地生命周期命令    | 禁用不支持模式/操作         |

错误返回原则：

- `message` 是安全的中文用户提示。
- `field` 只使用 Shell Bridge v1 的四个稳定字段名。
- 完整 source chain 只进 Logcat。
- native panic 根据当前操作映射到相应通用稳定错误，不新增未经契约确认的随机错误码。

## 16. 文件级实施清单

### 16.1 根 Rust workspace

- `Cargo.toml`
  - 增加 `android/native` member。
  - 增加经核验的 `jni`、日志依赖和 Tokio `sync/time` features。
- `Cargo.lock`
  - 锁定新增依赖。

### 16.2 shared

- `shared/src/config.rs`
  - 保持配置模型和默认值权威。
  - 如现有 `garde::Report` 不便安全映射，增加结构化校验问题访问 API。
- `shared/src/lib.rs`、测试和 shared 代码地图
  - 导出并验证新增公共校验能力。

### 16.3 core

- 新增 `core/src/local_service.rs`。
- `core/src/lib.rs` 导出 `start_local_service`、`RunningLocalService` 和错误类型。
- 调整/复用 `core/src/server.rs` 的 bind 与 serve 能力。
- 增加启动、端口占用、实际地址、正常停止、端口释放和异常退出测试。
- 更新 `docs/code-map/core.md`。

### 16.4 server

- `server/src/lib.rs` 改用 `start_local_service()`。
- `server/src/error.rs` 收敛重复错误包装。
- 更新 server 测试和 `docs/code-map/server.md`。

### 16.5 Android Rust adapter

- 新增 `android/native/Cargo.toml` 与 `src/**`。
- 增加宿主机 contract/config/error tests。
- 增加 ARM64 cargo-ndk check/build gate。

### 16.6 Android Kotlin

- 新增 `WineStockApplication.kt`。
- 新增 `core/LocalCoreRuntimeManager.kt` 等 native 适配类。
- `AndroidManifest.xml` 注册 Application。
- `ShellBridgeHost.kt` 改为异步分发、generation 防护和 manager 事件订阅。
- `RuntimeSnapshotFactory.kt` 从 manager 状态构造真实快照和动态 capabilities。
- `RuntimeConfigStore.kt` 支持版本迁移和提交失败回滚。
- 删除全量规则镜像 `RuntimeConfigValidator.kt`；如采用 native 缺失降级策略，新增职责受限的 `RemoteRuntimeConfigFallbackValidator.kt`。
- `MainActivity.kt` 只绑定 manager，不拥有 core 启停。
- 增加 Kotlin unit tests 与 instrumentation tests。

### 16.7 Android Gradle

- `app/build.gradle.kts` 固定 NDK、ABI 和 generated jniLibs。
- `buildSrc` 增加 Rust build/verify/archive verify 任务。
- APK 验证任务同时检查前端资源和 native `.so`。
- 发布流水线归档 native symbols。

### 16.8 frontend

Shell Bridge v1 已包含所需命令和状态，原则上不需要新增协议方法。只在联调发现缺口时调整：

- `starting/stopping/failed` 的展示；
- 动态 lifecycle capabilities；
- 本地地址切换后的 HTTP client/session 重置；
- `native_library_unavailable`、`port_in_use`、migration 等错误提示。

不得为方便而新增业务 JNI 调用。

### 16.9 文档与代码地图

实施完成后同步更新：

- `docs/platforms.md`；
- `docs/runtime-networking.md`；
- `docs/shell-bridge.md`（仅当契约细节确有变化）；
- `android/docs/README.md`；
- `docs/code-map/android.md`；
- `docs/code-map/core.md`；
- `docs/code-map/server.md`；
- 本实施方案的状态与实际验证记录。

## 17. 分阶段实施顺序

### 阶段 0：冻结契约与基线

- 记录当前 APK 大小、冷启动时间和 Android 远端模式 smoke。
- 固定 native protocol v1、错误码、ABI 和 NDK 版本。
- 确认当前已通过的 ARM64 core 交叉编译结果，并冻结仅 ARM64 的构建范围。

完成门槛：ARM64 core check 通过，且构建配置没有 32 位或 x86 ABI 分支。

### 阶段 1：core 运行句柄

- 实现 `RunningLocalService`。
- 覆盖启动、停止、端口释放和错误测试。
- 让 `server` 改用新 API。

完成门槛：server 行为不回退，公共 API 在宿主机测试通过。

### 阶段 2：native adapter

- 建立 `android/native`。
- 完成 engine、配置映射、错误映射和 JNI JSON contract。
- 在宿主机测试非 FFI 部分。
- 构建 ARM64 `.so`。

完成门槛：JNI 入口可加载，start/get state/stop 在 instrumentation 中通过。

### 阶段 3：Gradle 构建与打包

- 增加 variant-aware Rust 构建任务。
- 接入 generated jniLibs。
- 增加 ELF 与 APK 验证。

完成门槛：Debug APK 和 Release APK 都只包含正确的 ARM64 `.so`；目标库缺失或出现意外 ABI 时构建失败。

### 阶段 4：Application manager 与配置事务

- 新增 Application 级 manager。
- 完成 native 安全加载、存储目录、默认配置、start/stop/restart 和回滚。
- 删除 Kotlin 权威校验镜像。

完成门槛：Activity 重建不重启 core，端口占用和 migration 失败可恢复。

### 阶段 5：Shell Bridge 与前端联调

- 异步化 `ShellBridgeHost`。
- 发布真实快照和 capabilities。
- 验证地址切换、健康检查和会话重置。

完成门槛：WebView 完整使用本机 core，前端所有设置/错误路径可操作。

### 阶段 6：真实设备与发布验证

- ARM64 真实设备；当前阶段不要求或打包 x86/x86_64 模拟器 ABI。
- 冷启动、旋转、后台恢复、force-stop 后恢复。
- APK、native symbols、包体和资源占用记录。
- 更新代码地图与实施记录。

## 18. 验证矩阵

### 18.1 Rust

```text
cargo +stable fmt --all -- --check
cargo +stable check -p winestock-shared
cargo +stable check -p winestock-core
cargo +stable test -p winestock-core
cargo +stable check -p winestock-server
cargo +stable test -p winestock-server
cargo +stable test -p winestock-android-native
```

跨 target：

```text
cargo ndk -t arm64-v8a -P 26 check -p winestock-android-native --locked
```

由于新增 workspace member、公共 API 和依赖 features，最终还应执行一次 workspace 级完整检查。

### 18.2 core 生命周期

- 正常启动并访问 `/api/health`。
- `127.0.0.1:<port>` 实际绑定地址正确。
- 指定端口占用返回 `port_in_use`。
- 非法 bindHost 返回 `invalid_bind_host`。
- shutdown 后端口可被重新绑定。
- 连续 start/stop/restart 不泄漏 listener 或任务。
- serve task 意外结束可被观察为 `service_crashed`。

### 18.3 Android 单元测试

- native loader 成功/失败状态。
- native protocol 版本不匹配。
- 配置 missing/invalid/present/default-created。
- local、remote 和 unsupported server-mode 映射。
- apply 成功后提交。
- start 成功但配置保存失败时回滚。
- 候选启动失败时恢复旧服务。
- 重复 start/stop/restart 幂等。
- generation 变化后丢弃迟到回复。

### 18.4 Android instrumentation

- APK 能加载对应 ABI `.so`。
- 首次启动创建默认配置、数据库和文件目录。
- WebView 通过 HTTP 完成健康检查、登录和一个读写业务流程。
- 图片上传/下载经过本地 core。
- Activity 旋转前后 bound address 和 engine generation 不变。
- 页面 reload 不重启 core。
- 切换远端时本地端口释放。
- 切回 self-hosted 时本地服务恢复。
- 端口占用错误在前端可修改并重试。
- native load 失败的测试包仍能打开设置页并切远端。
- 非受信任 origin 无法调用 lifecycle bridge。

### 18.5 进程与恢复

- `adb shell am force-stop` 后重新启动，可重新打开数据库并恢复配置。
- 从最近任务移除后重新进入，状态符合当前进程策略。
- 后台短暂停留再恢复不重复 migration 或启动第二个 listener。
- 模拟系统杀进程后 SQLite WAL 能正常恢复。

### 18.6 构建产物

- Debug APK 的 ARM64 `.so` 完整，且不包含其它 ABI。
- Release APK 的 ARM64 `.so` 完整，且不包含其它 ABI。
- ABI 过滤与 cargo-ndk 目标一致。
- `.so` 没有未打包依赖。
- Release native symbols 可用于解析测试 tombstone。
- 记录引入 native 前后的 Debug/Release APK 大小变化。

### 18.7 性能与稳定性

- core bootstrap 不阻塞主线程或触发 ANR。
- 记录首次 migration 与后续冷启动耗时。
- 记录 Runtime 常驻线程数、空闲 CPU 和基础内存。
- 连续 20 次 restart 无端口残留、线程持续增长或数据库锁死。
- 前端首屏可以在 core 启动期间显示 `starting`，不因 API 尚未可用白屏。

## 19. 主要风险与缓解

| 风险                                 | 缓解措施                                                         |
| ------------------------------------ | ---------------------------------------------------------------- |
| JNI panic 导致进程崩溃               | 每个入口 `catch_unwind`，只返回稳定 JSON 错误                    |
| Activity 重建导致重复服务            | Application 级单例 manager，Activity 只订阅                      |
| migration 阻塞 UI                    | Kotlin 单线程后台 executor + 长期 Tokio Runtime                  |
| Tokio Runtime 嵌套 panic             | 只从非 runtime JNI executor 调用 `block_on`                      |
| 配置保存成功但服务失败               | 激活成功后提交；失败恢复旧配置/旧服务                            |
| 配置保存失败但候选服务已运行         | 关闭候选并回滚                                                   |
| `.so` ABI 缺失                       | generated jniLibs + APK 包级验证                                 |
| 工具链自动联网或构建长时间等待       | 显式环境准备，普通构建使用 `--locked --offline`                  |
| Kotlin 与 shared 校验漂移            | 删除规则镜像，native/shared 权威校验                             |
| 进程被系统杀死无法 graceful shutdown | 接受 Android 进程模型，依赖 WAL 恢复并在下次冷启动重建           |
| self-hosted 意外暴露局域网           | Android 第一阶段只允许 loopback                                  |
| 后台 server 不符合系统规则           | `server-mode` capability 保持 false，后续单独实现 FGS            |
| 本地 HTTP 被 WebView 安全策略拦截    | 保留 cleartext/mixed-content 配置并做真实 WebView/CORS/PNA smoke |
| Rust 包体较大                        | 先记录仅 ARM64 APK 的真实增量，再决定 feature-gate/strip         |
| 页面重载收到旧异步回复               | request generation + 当前 reply proxy 校验                       |

## 20. 完成定义

只有同时满足以下条件，才能认为 Android 已正式引入 core：

- `android/native` 作为唯一 JNI 适配层依赖 `winestock-core`。
- `core` 提供统一可停止运行句柄，`server` 已复用。
- Android `self-hosted` 冷启动可运行真实 core HTTP API。
- Activity 重建不影响应用级服务。
- 配置切换、端口占用、存储/migration 和 native load 失败均有可修复前端状态。
- Kotlin 不再承担 shared 权威配置校验。
- APK 对 ARM64 `.so` 有自动化包级验证，并拒绝意外 ABI。
- 未实现 Foreground Service 前 `server-mode` 仍不可用。
- 业务能力继续全部通过 HTTP，没有新增业务 JNI 接口。
- Rust、Kotlin、Gradle、WebView 和真实设备验证结果已写回相关文档与代码地图。

## 21. 推荐实施决策汇总

| 决策点             | 推荐结论                                   |
| ------------------ | ------------------------------------------ |
| Rust/Kotlin 桥     | 具名 jni-rs + JSON，不使用 UniFFI          |
| JNI 所在位置       | 独立 `android/native`，不进入 core/shared  |
| Tokio 生命周期     | Application 进程级长期 Runtime             |
| core 生命周期 API  | `RunningLocalService` 唯一句柄             |
| Android 服务所有者 | `LocalCoreRuntimeManager`，不是 Activity   |
| 配置持久化         | 第一阶段沿用版本化 SharedPreferences       |
| 权威配置校验       | `winestock_shared`                         |
| 本地存储           | `noBackupFilesDir/winestock/data`          |
| self-hosted 绑定   | 第一阶段强制 `127.0.0.1`                   |
| ABI                | 当前阶段仅 `arm64-v8a`                     |
| 最低 API           | 与 Android 工程一致的 26                   |
| server-mode        | FGS 完成前禁用                             |
| 业务调用           | WebView -> HTTP -> core，不走 JNI          |
| native 失败策略    | 前端仍可加载并切远端，不因静态加载直接崩溃 |

## 22. 实施与验证记录

### 22.1 已完成实现

- `shared` 提供结构化 `validation_issues()`，JNI 不解析展示文本。
- `core` 提供 `start_local_service()` 与 `RunningLocalService`；`server` 已复用。
- `android/native` 已实现 jni-rs 0.22 JSON protocol v1、Tokio Runtime、配置映射和稳定错误。
- Application 级 `LocalCoreRuntimeManager` 已实现默认启动、配置事务、回滚和远端降级。
- `ShellBridgeHost` 已异步化并增加页面 generation 防护；原生返回 broker/fallback 保持原行为。
- Gradle 已固定 NDK `30.0.14904198`、`cargo-ndk 4.1.2`、唯一 ABI `arm64-v8a`。
- Debug 使用 Cargo debug profile；Release 明确使用 `cargo ndk ... --release`，因此
  `winestock-android-native -> winestock-core -> winestock-shared` 整条依赖链均按 release profile 编译。
- `utoipa-swagger-ui` 启用 vendored 资源，普通 APK 构建不再由 Rust build script 联网下载 Swagger UI。
- 当前只存在 APK 包级验证，不存在自定义 AAB 校验或 bundle 挂钩。

### 22.2 已执行验证

```text
cargo +stable fmt --all -- --check                     # passed
cargo +stable check --workspace --all-targets          # passed
cargo +stable test --workspace                         # native 5 / core 101 / server 6 / shared 9 passed
cargo ndk -t arm64-v8a -P 26 check ... --locked       # passed
cargo ndk -t arm64-v8a -P 26 build ... --locked       # passed
gradlew.bat :app:testDebugUnitTest                     # 21 passed
gradlew.bat :app:lintDebug                             # 0 errors, 10 warnings
gradlew.bat :app:assembleDebug                         # passed, profile=debug
gradlew.bat :app:assembleRelease                       # passed, profile=release
```

首次 APK 结果：

| 制品 | APK 大小 | 包内 native library | ABI |
| ---- | -------- | ------------------- | --- |
| Debug | 28,320,983 bytes | 20,919,752 bytes | 仅 `arm64-v8a` |
| Release unsigned | 29,842,036 bytes | 23,885,144 bytes | 仅 `arm64-v8a` |

两个 APK 均通过最终归档检查，目标路径为：

```text
lib/arm64-v8a/libwinestock_android_native.so
```

### 22.3 已完成的最小真机 smoke

2026-07-23 在已连接的 ARM64 真机 `M2012K11AC` 上完成以下最小验证：

- 使用 `adb install -r` 覆盖安装 Debug APK，未清除已有应用数据；
- 冷启动 `MainActivity` 成功，Activity 保持 resumed，未出现 AndroidRuntime/FATAL 崩溃；
- Logcat 出现 `JNI_OnLoad success`，证明 APK 内 ARM64 `.so` 已由设备实际加载；
- 从已有远端配置切换到 `self-hosted` 后，配置事务成功持久化；
- Rust 端完成 SQLite migration 检查，设备内请求 `http://127.0.0.1:17890/api/health`
  返回 `{"status":"OK"}`。

后续冷启动恢复检查开始后，因其它任务正在使用同一测试环境而按用户要求停止；不得把未完成项目推断为通过。

### 22.4 待真实设备测试

以下项目仍未形成完整证据，统一标记为“待真实设备测试”：

- 登录、业务读写、图片上传和下载；
- WebView 在进程冷启动后自动使用恢复出的本地 API 地址；
- Activity 旋转、页面 reload、前后台恢复时 core generation 保持；
- 手势返回与三键返回；
- 远端/本地反复切换、失败回滚和端口占用恢复；
- force-stop、系统杀进程和 SQLite WAL 恢复；
- 冷启动耗时、内存、空闲 CPU 与连续 restart 稳定性。

## 23. 参考资料

- 项目架构：`docs/architecture.md`
- 项目结构：`docs/project-structure.md`
- 平台职责：`docs/platforms.md`
- 运行网络：`docs/runtime-networking.md`
- Shell Bridge：`docs/shell-bridge.md`
- Android 代码地图：`docs/code-map/android.md`
- core 代码地图：`docs/code-map/core.md`
- Android Developers：[Native library 与 ABI 配置](https://developer.android.com/ndk/guides/abis)
- Android Developers：[向项目添加 C/C++/native code](https://developer.android.com/studio/projects/add-native-code)
- Android Developers：[Foreground Service 后台启动限制](https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start)
- Tokio：[Runtime 文档（当前 lockfile 为 1.52.3）](https://docs.rs/tokio/1.52.3/tokio/runtime/struct.Runtime.html)
- [cargo-ndk](https://github.com/bbqsrc/cargo-ndk)
