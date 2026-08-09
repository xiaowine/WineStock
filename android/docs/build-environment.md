# Android 构建环境与工具链要求

本文件是 Android APK 构建的宿主机前置条件权威说明。

普通 Gradle 构建**不联网安装** Rust、rustup target、cargo-ndk、NDK 或前端依赖；缺失时构建立即失败并给出原因，不会静默联网补齐
（约束见 `../../docs/implementation-notes/android-embedded-core-integration.md` 的 "Gradle、cargo-ndk 与 ABI 打包" 一节）。
以下所有工具必须在首次构建前显式准备。

## 版本锁定点

| 工具 | 要求 | 校验方式 |
|---|---|---|
| Rust | ≥ 1.94（SeaORM 2.0 下限） | 不校验版本；低于下限时依赖编译失败。实现快照 `1.96.1`，CI 使用 `stable` |
| cargo-ndk | **4.1.2（精确）** | Gradle 构建前执行 `cargo ndk --version`，输出必须包含 `4.1.2`，否则任务失败 |
| rustup target | `aarch64-linux-android` | cargo 编译 Android target 时按需使用，缺失即编译失败 |
| Android NDK | **30.0.14904198（精确，AGP 完全匹配）** | `ndkVersion` 与已安装版本不一致时 AGP 抛 `NDK is not installed` |
| Cargo 依赖 | 已执行 `cargo fetch --locked` | 构建使用 `--locked --offline`，未准备依赖立即失败 |
| 前端依赖 | 已执行 `pnpm --dir frontend install --frozen-lockfile` | 见 [`README.md`](README.md) "前端资源打包" |

当前唯一 ABI 为 `arm64-v8a`，native 最低 API 26（Gradle `minApi`），app `minSdk` 28；不生成 32 位 ARM、x86 或 x86_64。

## 一次性准备步骤

```text
rustup target add aarch64-linux-android
cargo install cargo-ndk --version 4.1.2 --locked
cargo fetch --locked            # 在仓库根目录执行
```

### NDK 30.0.14904198 安装

- Android Studio → SDK Manager → SDK Tools → "NDK (Side by side)" → 勾选 **Show Package Details**，
  从版本列表选择 `30.0.14904198`（对应 `android-ndk-r30-beta1`）安装。
- SDK Manager 默认只显示最新版本。当前最新为 `30.0.15729638`（`android-ndk-r30-beta2`），
  与本项目锁定版本不一致，直接装默认项会导致 `NDK is not installed`。
- 命令行方式：`sdkmanager "ndk;30.0.14904198"`。

## 验证

```text
cargo ndk --version              # 必须包含 4.1.2
rustup target list --installed   # 必须包含 aarch64-linux-android
```

`app/build.gradle.kts` 的 `ndkVersion` 与 `cargoNdkVersion` 是权威来源；升级 NDK 或 cargo-ndk 必须先改配置与 CI，
再更新本表，不得只改本机安装。

## 常见失败对照

| 报错 | 原因 | 处理 |
|---|---|---|
| `NDK is not installed`（`:app:buildDebugRustNativeLibraries`） | `ndkVersion` 对应版本未安装 | 按上文安装 `30.0.14904198` |
| `error: no such command: 'ndk'` | cargo-ndk 未安装 | `cargo install cargo-ndk --version 4.1.2 --locked` |
| `cargo-ndk 版本不匹配：需要 4.1.2，实际输出为 …` | cargo-ndk 版本不符（Gradle 硬校验） | 重装精确版本 `4.1.2` |
| cargo 报 Android target 编译失败（找不到 target） | `aarch64-linux-android` target 未安装 | `rustup target add aarch64-linux-android` |
| `--offline` 下提示缺少 crate | 依赖未 fetch | 仓库根执行 `cargo fetch --locked` |
