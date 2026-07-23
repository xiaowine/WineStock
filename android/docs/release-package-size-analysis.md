# Android Release 包体分析

本文记录 2026-07-23 对 ARM64 Release APK 的实际构建与制品检查。当前交付范围仍是单 ABI APK，
不包含 AAB、32 位 ARM 或模拟器 ABI。

## 本次结论

Release core 已通过 `cfg(debug_assertions)` 完全移除 Swagger UI 路由和代码引用，只保留
`/api-docs/openapi.json`。Swagger UI vendored 文件仍可能在 Cargo 依赖的构建目录中生成，但不会链接进
Server Release 二进制、Android Release `.so` 或最终 APK。

实际重打包后，APK 从 29,842,036 bytes（28.46 MiB）降到 18,262,608 bytes（17.42 MiB），
减少 11,579,428 bytes（11.04 MiB，38.8%）。最终 APK 内的 native library 从
23,885,144 bytes（22.78 MiB）降到 12,305,672 bytes（11.74 MiB），减少 48.5%。

| 项目               |           修改前 |           修改后 |                        变化 |
| ------------------ | ---------------: | ---------------: | --------------------------: |
| Release APK        | 29,842,036 bytes | 18,262,608 bytes | -11,579,428 bytes（-38.8%） |
| APK 内 ARM64 `.so` | 23,885,144 bytes | 12,305,672 bytes | -11,579,472 bytes（-48.5%） |

APK 与 Server Release 二进制均未检出 `/swagger-ui`、`Swagger UI:`、
`swagger-ui-bundle.js` 或 source map 文件名。Release HTTP 测试同时确认 OpenAPI JSON 返回 200，
`/swagger-ui/` 返回统一 JSON 404。

## 当前 APK 组成

以下比例按 APK 文件 18,262,608 bytes 计算；native library 与 `resources.arsc` 在当前 APK 中未压缩。

| 内容                                           |       APK 中大小 | 占 APK |
| ---------------------------------------------- | ---------------: | -----: |
| `lib/arm64-v8a/libwinestock_android_native.so` | 12,305,672 bytes |  67.4% |
| Dex（压缩后）                                  |  3,864,935 bytes |  21.2% |
| `resources.arsc`                               |  1,301,540 bytes |   7.1% |
| 前端 67 个文件（压缩后）                       |    277,528 bytes |   1.5% |
| 其它文件与 ZIP 开销                            |    512,933 bytes |   2.8% |

Swagger UI 移除后，native library 仍是第一大项，但其主要内容已经从静态文档资源转为实际 Rust 代码。

## 当前 native library 组成

最终 APK 使用 Gradle strip 后的 12,305,672-byte ELF。主要段如下：

| ELF 段                                             |                        大小 | 占最终 `.so` |
| -------------------------------------------------- | --------------------------: | -----------: |
| `.text`                                            | 9,134,068 bytes（8.71 MiB） |        74.2% |
| 异常与 unwind（`.gcc_except_table`、`.eh_frame*`） | 2,033,056 bytes（1.94 MiB） |        16.5% |
| `.rodata`                                          |   589,000 bytes（0.56 MiB） |         4.8% |
| 重定位与只读数据（`.rela.dyn`、`.data.rel.ro`）    |   513,208 bytes（0.49 MiB） |         4.2% |

移除 Swagger UI 前 `.rodata` 为 11,873,032 bytes；当前只剩 589,000 bytes，说明此前约 11 MiB 的
只读数据基本就是嵌入的 Swagger UI 分发文件。

## 仍可评估的压缩方向

1. Rust Release profile 可评估 `opt-level = "z"`、Thin LTO 和 `codegen-units = 1`。此前隔离实验在仍含
   Swagger UI 时已把 `.text` 从约 9.23 MB 降到约 4.08 MB；该优化与本次资源移除可以叠加，但应重新
   跑启动、数据库、鉴权和 Android 生命周期回归后再作为正式 profile。
2. Android Release 当前明确关闭 `optimization`。启用 R8/minify 与 resource shrink 后，主要目标是
   当前压缩后约 3.69 MiB 的 Dex 和约 1.24 MiB 的资源表；需要完整验证 WebView、Shell Bridge、反射和
   AndroidX 行为，预期收益低于本次 Swagger UI 移除。
3. `panic = "abort"` 可能继续减少约 1.94 MiB 异常/unwind 数据中的一部分，但会把 Rust panic 改为直接
   终止进程，属于行为与故障恢复策略变化，不应只为包体直接启用。
4. 前端压缩后只有约 0.26 MiB，不是当前优化重点。

## 实际验证

- `cargo +stable test -p winestock-core http_openapi`
- `cargo +stable test -p winestock-core --release http_openapi`
- `cargo +stable check -p winestock-server`
- `cargo +stable build -p winestock-server --release`
- `android/gradlew.bat :app:assembleRelease --no-daemon --no-configuration-cache`
- Gradle `verifyReleaseRustNativeLibraries`、`verifyReleaseRustNativeApkPackage`、
  `verifyReleaseFrontendPackage` 与 `lintVitalRelease`
- `llvm-size --format=sysv` 检查最终 strip 后 ARM64 ELF
- 对 Android `.so` 与 Server Release 二进制执行 Swagger UI 路径、文件名和启动提示字符串扫描
