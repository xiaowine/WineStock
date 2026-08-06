# Android Release 包体分析

本文记录 2026-07-23 对 ARM64 Release APK 的实际构建与制品检查。当前交付范围仍是单 ABI APK，
不包含 AAB、32 位 ARM 或模拟器 ABI。

## 本次结论

Release core 已通过 `cfg(debug_assertions)` 完全移除 Swagger UI 与 OpenAPI JSON 路由。Android Release
Release 不注册 Swagger UI；`/api-docs/openapi.json`
和 `/swagger-ui` 均返回统一 JSON 404。

实际重打包后，APK 从 29,842,036 bytes（28.46 MiB）降到 12,373,642 bytes（11.80 MiB），
减少 17,468,394 bytes（16.66 MiB，58.5%）。最终 APK 内的 native library 从
23,885,144 bytes（22.78 MiB）降到 10,634,528 bytes（10.14 MiB），减少 55.5%。
其中 R8 minify/resource shrink 把 Swagger UI 与 SQLite FTS 已移除后的 APK 从
16,591,327 bytes 继续降到 12,373,642 bytes，主要收益来自 Dex 与资源表；同时排除了
`kotlinx-coroutines` 仅供调试器使用的 `DebugProbesKt.bin`。

| 项目               |           修改前 |           修改后 |                        变化 |
| ------------------ | ---------------: | ---------------: | --------------------------: |
| Release APK        | 29,842,036 bytes | 12,373,642 bytes | -17,468,394 bytes（-58.5%） |
| APK 内 ARM64 `.so` | 23,885,144 bytes | 10,634,528 bytes | -13,250,616 bytes（-55.5%） |

APK 与 Server Release 二进制均未检出 `/swagger-ui`、`Swagger UI:`、
`swagger-ui-bundle.js` 或 source map 文件名。Release HTTP 测试同时确认 OpenAPI JSON 与
`/swagger-ui/` 均返回统一 JSON 404。R8 `seeds.txt` 和 `dexdump` 同时确认
`winestock.xiaowine.cc.core.NativeCoreBridge` 及其 native 方法名保持稳定。

## 当前 APK 组成

以下比例按 APK 文件 12,373,642 bytes 计算；native library 与 `resources.arsc` 在当前 APK 中未压缩。

| 内容                                           |       APK 中大小 | 占 APK |
| ---------------------------------------------- | ---------------: | -----: |
| `lib/arm64-v8a/libwinestock_android.so` | 10,634,528 bytes |  85.9% |
| `resources.arsc`                               |    671,288 bytes |   5.4% |
| Dex（压缩后）                                  |    435,624 bytes |   3.5% |
| 前端 69 个文件（压缩后）                       |    277,065 bytes |   2.2% |
| 其它文件与 ZIP 开销                            |    355,137 bytes |   2.9% |

Swagger UI 移除后，native library 仍是第一大项，但其主要内容已经从静态文档资源转为实际 Rust 代码。
R8 后 Android Java/Kotlin 层已不是主要包体来源。

## 当前 native library 组成

最终 APK 使用 Gradle strip 后的 10,634,528-byte ELF。主要段如下：

| ELF 段                                             |                        大小 | 占最终 `.so` |
| -------------------------------------------------- | --------------------------: | -----------: |
| `.text`                                            | 7,654,748 bytes（7.30 MiB） |        72.0% |
| 异常与 unwind（`.gcc_except_table`、`.eh_frame*`） | 1,889,988 bytes（1.80 MiB） |        17.8% |
| `.rodata`                                          |   537,928 bytes（0.51 MiB） |         5.1% |
| 重定位与只读数据（`.rela.dyn`、`.data.rel.ro`）    |   515,688 bytes（0.49 MiB） |         4.9% |

移除 Swagger UI 前 `.rodata` 为 11,873,032 bytes；当前只剩 589,448 bytes，说明此前约 11 MiB 的
只读数据基本就是嵌入的 Swagger UI 分发文件。

按 `llvm-nm --print-size --size-sort --demangle` 对未 strip 的 Release `.so` 做启发式归类后，`.text`
中主要内容如下。Rust 泛型单态化会让符号归属互相交叉，因此该表用于定位方向，不能直接等同于删除某个
crate 后的收益。

| 内容归类                     | 估算大小 |
| ---------------------------- | -------: |
| 其它泛型实例和未细分依赖     | 1.48 MiB |
| SeaORM / SeaQuery            | 1.40 MiB |
| WineStock core / JNI adapter | 1.09 MiB |
| Utoipa / OpenAPI 生成代码    | 1.05 MiB |
| SQLite C core / FTS          | 0.94 MiB |
| Axum / Hyper / HTTP          | 0.89 MiB |
| Rust std / core / backtrace  | 0.57 MiB |
| Serde / JSON                 | 0.54 MiB |
| JWT / 密码学 / Argon2        | 0.38 MiB |
| SQLx                         | 0.36 MiB |
| Tokio / Mio                  | 0.20 MiB |
| JNI / Android logger         | 0.10 MiB |
| URL / IDNA / ICU / Regex     | 0.08 MiB |

观察到的大符号包括：

- Debug 文档构建中 `winestock_core::http::docs::ApiDoc::openapi` 约 59.7 KiB；Release 已不注册该路由。
- SQLite 的 `sqlite3VdbeExec`、`sqlite3Select`、`sqlite3Pragma`、FTS3/FTS5、JSON 和 RTREE 符号。
- 大量 Axum handler 泛型实例、SeaORM/SeaQuery 查询构造、SQLx 连接池和 SQLite explain 代码。
- `jsonwebtoken` 当前只被业务用于 HS256，但 `rust_crypto` feature 会启用 HMAC、RSA、P256、P384、
  Ed25519 和 SHA2；Release `.so` 中也实际出现了 RSA、P256、P384、Ed25519/Curve25519 符号。

## 仍可评估的压缩方向

1. 工作区 Release profile 现已启用 fat LTO，并显式关闭 Cargo strip；仍不采用 `opt-level = "z"`，也不
   限制 `codegen-units`。此前隔离构建验证显示，只启用 fat LTO、保持默认 `opt-level=3` 时，Android
   打包流程 strip 后 `.so` 为 12,339,360 bytes，比当时默认 Release 多约 1.1 KiB，因此该配置不应被视为
   Android 包体压缩措施。
2. SQLite bundled 构建原先固定启用了 FTS3、FTS5、JSON、RTREE、STAT4 等能力。项目当前使用
   `json_extract` 查询动态属性，因此 JSON 不能关闭；源码未使用 FTS3/FTS5。Android Release 构建已通过
   `LIBSQLITE3_FLAGS` 取消 FTS3/FTS5；最终 `.so` 字符串与符号扫描未检出 FTS3/FTS5 入口。
3. `jsonwebtoken` 的 `rust_crypto` backend 会把 RSA、P256、P384、Ed25519 等非当前业务算法一起带入。
   若长期只支持 HS256，可评估自定义 `CryptoProvider` 或改用更窄的 HMAC JWT 实现。该方向属于鉴权实现
   变更，必须覆盖登录、刷新、登出、过期、错误 token、旧 token 兼容和 Android 本地启动回归。
4. Android Release 已启用 R8 minify、optimize 与 resource shrink，并通过 `src/main/keepRules/rules.keep`
   保留 JNI 边界。该项把 Dex 压缩后大小降到 435,624 bytes，`resources.arsc` 降到 671,288 bytes；仍需
   真机覆盖 WebView、Shell Bridge、自托管 core 启动和远端模式。
5. `panic = "abort"` 可能继续减少约 1.94 MiB 异常/unwind 数据中的一部分，但会把 Rust panic 改为直接
   终止进程，属于行为与故障恢复策略变化，不应只为包体直接启用。
6. Android Release 已不公开 `/api-docs/openapi.json`。OpenAPI 相关宏仍存在于业务 DTO 和 handler 标注中；
   若要进一步完全移除 Utoipa 派生和 path 宏，需要把所有 `utoipa::ToSchema` 与 `#[utoipa::path]` 做
   feature 化，改动面会覆盖大部分 controller DTO。
7. 前端压缩后只有约 0.26 MiB，不是当前优化重点。

## 实际验证

- `cargo +stable test -p winestock-core http_openapi`
- `cargo +stable test -p winestock-core --release http_openapi`
- `cargo +stable check -p winestock-server`
- `cargo +stable build -p winestock-server --release`
- `android/gradlew.bat :app:assembleRelease --no-daemon --no-configuration-cache`
- Gradle `verifyReleaseRustNativeLibraries`、`verifyReleaseRustNativeApkPackage`、
  `verifyReleaseFrontendPackage`、`minifyReleaseWithR8`、`convertShrunkResourcesToBinaryRelease`
  与 `lintVitalRelease`
- `llvm-size --format=sysv` 检查最终 strip 后 ARM64 ELF
- 对 Android `.so` 与 Server Release 二进制执行 Swagger UI 路径、文件名和启动提示字符串扫描
- 对 Release APK 执行 ZIP 组成统计、R8 `seeds.txt` 检查与 `dexdump` JNI 类/方法名检查
- 对 Release APK 检查确认 `DebugProbesKt.bin` 已由 packaging resources exclude 移除
- `cargo ndk -t arm64-v8a -P 26 ... --release` 隔离验证 fat LTO 对照
- `cargo ndk -t arm64-v8a -P 26 ... --release` 隔离验证 SQLite FTS 关闭对照
- `cargo tree -p winestock-core -e features --offline --locked`
- `cargo info jsonwebtoken --offline`
- `cargo info sqlx-sqlite --offline`
- `cargo info libsqlite3-sys --offline`
