# WineStock Android 前端打包工作流完善方案

> 文档状态：已实施；API 33 真机离线运行 smoke 已完成，CI frozen install 与跨系统确定性验证待执行<br>
> 涉及组件：`android`、`frontend`、CI/发布流程<br>
> 编制日期：2026-07-23<br>
> 适用范围：当前 Android Gradle Plugin `9.2.1`、Gradle `9.6.1`、Vue/Vite 前端

## 1. 结论

Android 前端打包应从“发现已有 `frontend/dist` 后复制”升级为“由 Android 构建图生成、校验并消费当前前端产物”。

目标工作流为：

1. CI 或开发者先用锁文件准备前端依赖；常规 Android 构建不隐式联网安装依赖。
2. `assemble<Variant>`、`bundle<Variant>` 和 `install<Variant>` 自动触发当前源码的前端类型检查与 Vite 构建。
3. Vite 为 Android 使用独立、受控的构建配置，输出到 `android/app/build/`，不再把 `frontend/dist` 当作 Android 输入。
4. Gradle 对前端源码、配置、锁文件和构建模式声明输入，对生成目录声明输出，使未变更构建可进入 `UP-TO-DATE`。
5. 生成产物必须通过入口、manifest、引用完整性、开发服务器残留和环境污染检查；任何失败都中止 Android 构建。
6. Android Gradle Plugin 以“生成的 assets 目录”消费校验后的资源，不再向 `src/main/assets/frontend` 写生成文件，也不再通过全局 `preBuild` 粗粒度挂接。
7. CI 在 APK/AAB 层再次确认实际包内的 `assets/frontend` 与本次已校验产物一致。

最终必须满足：**没有当前前端构建，就没有可成功产出的 Android 包；绝不回退到旧 assets。**

## 2. 当前实现与问题

### 2.1 当前流程

当前 `android/app/build.gradle.kts` 在 Gradle 配置期检查 `frontend/dist`：

```text
frontend/dist 是否为目录
  -> 是：启用 syncFrontendAssets
       -> Sync 到 app/src/main/assets/frontend
  -> 否：禁用任务

preBuild
  -> dependsOn(syncFrontendAssets)
```

相关实现位于：

- `android/app/build.gradle.kts:45-56`：目录存在判断、`Sync` 和 `preBuild` 挂接。
- `android/app/.gitignore:3-4`：忽略生成在源码树中的 `assets/frontend`。
- `frontend/package.json:8`：前端构建只由人工执行 `pnpm build`。
- `frontend/vite.config.ts`：只有 Vue 插件，没有 Android 构建模式、产物 manifest 或环境隔离。

### 2.2 正确性缺口

| 问题                 | 当前行为                          | 风险                                             |
| -------------------- | --------------------------------- | ------------------------------------------------ |
| Android 不构建前端   | 只消费已有 `dist`                 | 前端源码已变化但 APK 仍携带旧页面                |
| 缺失时静默跳过       | `enabled = frontendDistExists`    | 旧 `assets/frontend` 仍可继续参与打包            |
| 只判断目录存在       | 不检查入口、资源引用或来源        | 空目录、半成品和失败后残留可能被接受             |
| 生成物写入源码树     | 输出在 `src/main/assets/frontend` | `gradlew clean` 不负责清理，IDE 与构建所有权模糊 |
| 全局挂到 `preBuild`  | 非 variant-aware                  | 调试、发布及未来不同模式无法独立约束             |
| 未声明完整输入/输出  | Gradle 不理解前端源码关系         | 无可靠增量判断，也不能安全使用构建缓存           |
| 构建命令未纳入任务图 | Android 不直接执行 pnpm           | 工具缺失或不兼容时无法在正确任务位置给出失败     |
| 环境变量未隔离       | Vite 会加载 `.env.local`          | 开发者本地 API 地址或元数据可能进入 APK          |
| 没有包级验证         | 只验证复制任务本身                | 无法证明最终 APK/AAB 确实包含本次产物            |

当前工作区检查还发现：

- `frontend/dist` 与 `android/app/src/main/assets/frontend` 可同时存在且更新时间不同，证明“前端构建”和“Android 同步”目前可以脱节。
- 工作区存在被忽略的 `frontend/.env.local`。本文未读取其内容，但 Vite 官方规则会在所有 mode 下加载 `.env.local`，因此仅增加 `--mode android` 仍不足以保证 Android 包不受本机配置影响。

### 2.3 根因

根因不是复制命令本身，而是当前没有把前端产物建模为 Android 构建图中的正式生成物：

- 没有生产者任务；
- 没有可追踪输入；
- 没有经过验证的输出；
- 没有由 Android variant 消费的 provider；
- 没有禁止旧产物回退的失败策略。

## 3. 目标与非目标

### 3.1 目标

- Android 构建自动使用当前前端源码，不要求人工先复制资源。
- 缺少 Node、pnpm、依赖或构建失败时给出明确错误并停止打包。
- 未变化的前端在连续 Android 构建中不重复执行 Vite。
- 前端构建环境不继承开发者的 `VITE_*` 或 `.env.local` 配置。
- 生成资源只存在于 Android `build/`，由 `gradlew clean` 统一清理。
- 调试包和发布包使用明确的 variant 策略，发布包具有更严格的校验。
- CI 能验证最终 APK/AAB 内资源，而不只验证中间目录。
- 保持现有 `https://winestock.internal/`、`assets/frontend`、`FrontendPathHandler` 和 Shell Bridge 运行边界不变。

### 3.2 非目标

- 不让 Axum 托管或打包前端资源。
- 不修改业务 HTTP API、鉴权、数据库或运行配置契约。
- 不在本次实现 Android 端本地 Axum。
- 不自动把 Vite dev server 暴露给正式 WebView origin。
- 不提交 `dist` 或其它生成前端文件到 Git。
- 不在每次 Android 构建中隐式联网执行 `pnpm install`。
- 不仅依靠文件时间戳判断“是否最新”；时间戳不能表达真实输入关系。

## 4. 职责边界

### 4.1 `frontend` 负责

- Vue/TypeScript 源码和类型检查。
- Vite Android 构建配置。
- 产出 `index.html`、hash 资源和构建 manifest。
- 定义允许进入客户端 bundle 的显式环境变量边界。
- 保持普通浏览器开发使用的 `pnpm dev` 和标准 `pnpm build`。

### 4.2 `android` 负责

- 决定何时为 Android variant 构建前端。
- 校验 Node/pnpm 前置条件并执行前端脚本。
- 声明 Gradle 任务输入、输出、依赖关系和生成目录。
- 校验、暂存并将前端资源注册为 Android generated assets。
- 验证 APK/AAB 中最终打包结果。

Android 不复制前端业务逻辑，也不维护第二份前端源码。

### 4.3 CI/发布流程负责

- 提供可直接执行的本机或 CI Node 与 pnpm 环境。
- 使用 `pnpm-lock.yaml` 以 frozen 模式准备依赖。
- 缓存 pnpm store 与 Gradle cache，不缓存或复用未经任务图校验的 `dist`、`src/main/assets/frontend`。
- 执行最终包验证和必要的 Android 启动 smoke test。

## 5. 目标构建图

```text
assemble<Variant> / bundle<Variant> / install<Variant>
  -> Android merge/package assets
     -> stage<Variant>FrontendAssets
        -> verify<Variant>FrontendAssets
           -> build<Variant>Frontend
           -> verifyNoLegacyFrontendAssets
```

关键点：

- Android Components Variant API 将 `stage<Variant>FrontendAssets` 的输出目录注册为 generated assets。
- 任务 provider 本身建立依赖，不再通过 `preBuild` 给所有构建挂一个无差别前置任务。
- 如果所有 variant 暂时使用完全相同的生产前端，可以让它们共享一个 `buildAndroidFrontend` 生产者；只有 mode、环境或 source map 策略真正不同后才拆成独立输出。
- 发布 variant 额外依赖包级验证任务。

## 6. 产物目录设计

建议目录：

```text
frontend/
  src/...
  public/...
  package.json
  pnpm-lock.yaml
  vite.config.ts

android/app/
  src/main/assets/
    shell/android-transport.js         # Android 平台源码资源，继续纳入版本库
  build/
    intermediates/winestockFrontend/android/dist/
      index.html
      asset-manifest.json
      assets/...
      favicon.svg
      icons.svg
    generated/winestockFrontendAssets/<variant>/
      frontend/
        index.html
        asset-manifest.json
        assets/...
```

规则：

- Vite 输出到 `build/intermediates`，验证通过后由 `Sync` 暂存到 `build/generated`。
- Android generated assets 的根目录是 `<variant>/`，其中保留 `frontend/` 子目录，因此运行时路径仍是 `assets/frontend/...`。
- `src/main/assets` 只保存平台拥有、需要版本控制的 `shell/android-transport.js` 等静态资源。
- 迁移时删除本机旧 `src/main/assets/frontend`，并增加守卫：该目录再次出现时构建直接失败，避免来源优先级不明确。
- `frontend/dist` 可以继续作为独立 Web 构建输出，但 Android 永不读取它。

## 7. 前端工具链与依赖准备

### 7.1 直接使用本机工具链

根据 2026-07-23 的实施确认，本项目不固定、不读取也不记录 Node 与 pnpm 版本：

- `frontend/package.json` 不声明 `packageManager` 或 Node/pnpm `engines`；
- 不增加 `.node-version`、`engine-strict` 或 Corepack 版本切换配置；
- Gradle 默认从当前进程的 `PATH` 直接执行 `pnpm run build:android`，pnpm 使用本机 Node；
- Android 构建不下载、不安装、不切换 Node 或 pnpm；
- pnpm 不存在、Node 与当前依赖不兼容或脚本执行失败时，由实际构建命令直接失败。

构建任务仍在执行命令前确认 `frontend/package.json`、`pnpm-lock.yaml` 和本地
`node_modules/.modules.yaml` 存在，依赖未准备时提示执行 frozen install，而不是尝试旧产物。

### 7.2 不在常规 assemble 中隐式安装

推荐命令边界：

```text
环境准备（显式、允许联网）
  -> pnpm --dir frontend install --frozen-lockfile

Android 构建（不主动联网安装）
  -> android/gradlew :app:assembleDebug
```

原因：

- Android 构建不应因为本地依赖缺失而突然访问网络。
- frozen install 应由 CI 和开发环境准备步骤明确执行，便于缓存和故障诊断。
- `node_modules` 不适合作为巨大的 Gradle 文件输入；锁文件和成功执行的当前前端构建共同保证依赖语义。

可以提供一个未挂入 `assemble` 的显式便利任务，例如 `prepareFrontendDependencies`，但它只能作为人工入口，不能成为发布构建的隐式网络副作用。

## 8. Vite Android 构建配置

### 8.1 独立脚本

建议保留现有标准脚本，并新增 Android 专用脚本：

```json
{
  "scripts": {
    "build": "vue-tsc -b && vite build",
    "build:android": "vue-tsc -b && vite build --mode android"
  }
}
```

Gradle 在 `frontend/` 目录执行 `pnpm run build:android`，并通过非 `VITE_` 前缀的进程变量传入输出目录。

### 8.2 明确 Android 配置

Android mode 应显式设置：

- `base: "/"`：与当前 `https://winestock.internal/` 根路径和 `FrontendPathHandler` 的映射契约一致。
- `build.outDir`：使用 Gradle 提供的 `build/intermediates/.../dist` 绝对路径。
- `build.emptyOutDir: true`：输出位于 Vite 根目录之外时仍先清理旧文件。
- `build.manifest: "asset-manifest.json"`：生成可机器校验的入口、动态 import、CSS 和静态资源映射。
- `build.sourcemap: false`：发布包不携带源码映射；调试包若未来确有需求，应作为显式 variant 策略。
- `envDir: false`，或指向只包含受版本控制配置的 Android 专用目录；首选 `false`。

Android 构建不需要通过 Vite 固化 `VITE_API_BASE_URL`、设备名或应用版本：这些值当前由 Shell Bridge/运行时注入，继续保持运行时所有权。

### 8.3 隔离进程环境

仅设置 `envDir: false` 仍不能阻止父进程已有的 `VITE_*` 环境变量。Gradle 执行前端任务时还应：

- 从子进程环境中移除继承的所有 `VITE_*` 变量；
- 显式设置 `NODE_ENV=production`；
- 只传入非客户端暴露前缀的构建控制变量，例如 `WINESTOCK_FRONTEND_OUT_DIR`；
- 不把签名、凭据、路径口令或服务 token 传给 Vite。

这样即使开发者存在 `.env.local` 或 shell 级 `VITE_API_BASE_URL`，Android APK 也不会受其影响。

## 9. Gradle 任务建模

### 9.1 任务实现位置

当前实现把非平凡任务类型放在 Android 自己的 `buildSrc` 构建逻辑中：

```text
android/buildSrc/src/main/kotlin/winestock/build/
  FrontendPackagingTasks.kt
  FrontendAssetValidation.kt
```

这些构建类型属于 `android`，不放入 `core`、`shared` 或前端运行时代码。

### 9.2 `buildAndroidFrontend`

输入至少包括：

- `frontend/package.json`；
- `frontend/pnpm-lock.yaml`；
- `frontend/vite.config.ts`；
- `frontend/index.html`；
- `frontend/tsconfig*.json`；
- `frontend/src/**`；
- `frontend/public/**`；
- 前端构建脚本、Android 构建 mode 和是否生成 source map 等任务属性。

显式排除：

- `frontend/node_modules/**`；
- `frontend/dist/**`；
- `.env.local` 和其它不应影响 Android 的本机配置；
- 日志、编辑器文件和临时输出。

输出：

- `android/app/build/intermediates/winestockFrontend/android/dist`。

任务使用 lazy `DirectoryProperty`、`RegularFileProperty` 和 provider 传递路径，避免在配置期读取或决定产物是否存在。外部命令返回非零、工具不存在或输出未生成时，任务必须失败。

### 9.3 增量与构建缓存

第一阶段目标：

- 正确声明 inputs/outputs；
- 连续未变更构建显示 `UP-TO-DATE`；
- 支持 Gradle configuration cache。

按当前决策，Node/pnpm 版本不是任务输入；本机工具发生变化时如需强制重新生成，应执行
`buildAndroidFrontend --rerun-tasks` 或清理 Android build 目录。共享远程构建缓存因此保持禁用。

不要在第一阶段直接把外部前端构建标记为可共享远程缓存。只有在 Windows/Linux、不同工作区绝对路径和干净环境下证明产物字节稳定后，再启用 `@CacheableTask` 或 `outputs.cacheIf`。

启用远程缓存前必须确认：

- 输出不包含时间戳；
- 输出不包含绝对路径；
- source map 未泄漏工作区路径；
- 构建环境标识已有明确且经用户确认的建模策略；
- 相同输入在受支持平台生成相同文件集和内容。

### 9.4 `verify<Variant>FrontendAssets`

该任务读取 Vite 输出并写入一个验证 marker，至少检查：

1. `index.html`、`asset-manifest.json` 与 `assets/` 存在且非空。
2. manifest 是合法 JSON，并包含至少一个入口。
3. manifest 中的 entry、CSS、imports、dynamic imports 和静态资源路径都存在于输出目录内。
4. `index.html` 的本地 `src`、`href` 引用都能在输出目录解析到真实文件。
5. 路径不能逃出输出根目录，也不能包含本机绝对路径。
6. 不存在 `/@vite/client`、Vite HMR websocket 或明确的开发服务器入口。
7. 发布构建不包含 `.map` 文件。
8. 文件总数和总体积非零；异常大文件先给出明确诊断，是否设硬上限由实施时结合 APK 预算决定。

验证 marker 应包含 manifest 内容摘要，而不是当前时间，以免破坏可复现性。

### 9.5 `stage<Variant>FrontendAssets`

该任务：

- 依赖验证任务；
- 使用 `Sync` 把已验证 dist 同步到 generated assets 下的 `frontend/`；
- 每次执行都删除目标中不再存在的旧 hash 文件；
- 输出目录通过 Android Components Variant API 注册给对应 variant。

因为 `Sync` 目标位于 `build/`，任务被跳过或失败时 Android 不存在可回退的源码树旧副本。

### 9.6 禁止旧目录

增加 `verifyNoLegacyFrontendAssets`：

- 如果 `android/app/src/main/assets/frontend` 存在，直接失败并提示删除；
- 从 `android/app/.gitignore` 移除对该目录的忽略，避免以后误生成而不被发现；
- 保留 `android/app/src/main/assets/shell`，它是 Android 平台源码资源，不是前端构建输出。

## 10. Variant 策略

### 10.1 第一阶段

第一阶段建议 Debug 与 Release 都打包生产形态的静态前端：

- 都执行类型检查与 Vite production build；
- 都从受信任本地 origin 加载；
- 都不依赖开发服务器；
- Release 追加更严格的 source map、manifest 与最终包检查。

如果两者输入完全相同，可共享一次前端生产任务以减少构建时间；Android variant 只分别暂存和验证最终包。

### 10.2 暂不引入 Android dev-server 模式

Android WebView 当前 Shell Bridge 明确限制在 `https://winestock.internal`。直接把 Debug WebView 指向局域网 Vite dev server 会同时改变：

- 受信任 origin；
- bridge 暴露边界；
- mixed-content 与网络安全策略；
- HMR websocket 行为；
- 离线启动能力。

因此本方案不顺手加入 dev-server 模式。若未来确实需要，应单独设计 Debug-only origin、桥禁用或严格 allowlist、设备/主机寻址与安全验收，不能把它混入正式打包任务。

## 11. 最终包验证

只验证 generated assets 还不足以证明 APK/AAB 正确。应增加 `verify<Variant>FrontendPackage`，读取 Android 构建产物并检查：

```text
assets/shell/android-transport.js
assets/frontend/index.html
assets/frontend/asset-manifest.json
assets/frontend/assets/...
```

验证内容：

- 包内 manifest 与暂存阶段 manifest 摘要一致；
- 包内所有 manifest 引用存在；
- 包内不存在第二份旧入口或 legacy frontend 路径；
- Release 包不包含 source map、Vite dev client 或测试专用资源；
- Shell Bridge 平台资源与前端资源同时存在。

APK 与 AAB 都是 ZIP 容器，可在 Gradle 自定义任务中使用 Java ZIP API 完成，不需要依赖外部解压命令。发布流水线必须运行该任务；Debug 可在 PR 中运行一次作为快速 gate。

## 12. 失败策略

| 场景                                   | 目标行为                                   |
| -------------------------------------- | ------------------------------------------ |
| 本机 Node 无法供 pnpm 使用             | 前端命令失败，不尝试读取旧 dist            |
| 本机 pnpm 不存在或无法执行             | 前端命令失败，不尝试读取旧 dist            |
| `node_modules` 未准备                  | 前端任务失败，提示 frozen install          |
| TypeScript/Vite 构建失败               | Android variant 立即失败                   |
| Vite 未生成入口或 manifest             | 校验失败                                   |
| manifest 引用文件缺失                  | 校验失败                                   |
| 本机 `.env.local` 存在                 | Android 构建忽略，不因此失败               |
| shell 中存在 `VITE_*`                  | 子进程移除，不能影响 bundle                |
| legacy `src/main/assets/frontend` 存在 | 构建失败，禁止来源混合                     |
| 前端源码变化                           | 重新执行前端构建与验证                     |
| 前端输入未变化                         | 前端生成任务 `UP-TO-DATE` 或受信任缓存命中 |
| 最终 APK/AAB 缺少或篡改资源            | 包级验证失败                               |

## 13. CI 与开发者工作流

### 13.1 本地首次准备

```text
1. 确认本机 `node` 与 `pnpm` 命令可用
2. `pnpm --dir frontend install --frozen-lockfile`
3. `android/gradlew :app:assembleDebug`
```

之后只要依赖锁文件未变，普通 Android 构建会按 Gradle 输入自动决定是否重建前端。

### 13.2 PR/持续集成

建议最小流水线：

```text
pnpm --dir frontend install --frozen-lockfile
android/gradlew :app:assembleDebug :app:verifyDebugFrontendPackage --configuration-cache
```

CI 缓存：

- pnpm store；
- Gradle user home/cache；
- 在完成确定性验证后再缓存前端 Gradle task output。

CI 不缓存：

- `frontend/dist` 作为 Android 输入；
- `android/app/src/main/assets/frontend`；
- 未经 inputs/outputs 建模的任意复制目录。

### 13.3 发布

发布流水线使用干净 checkout，并执行：

```text
pnpm --dir frontend install --frozen-lockfile
android/gradlew :app:bundleRelease :app:verifyReleaseFrontendPackage --configuration-cache
```

随后在模拟器或真实设备完成离线启动 smoke：即使 API 不可用，也必须能从包内资源打开前端和运行设置入口。

## 14. 预计文件变更

| 文件/目录                      | 变更                                                                        |
| ------------------------------ | --------------------------------------------------------------------------- |
| `frontend/package.json`        | 新增 `build:android`，不固定本机 Node/pnpm 版本                             |
| `frontend/vite.config.ts`      | 增加 Android mode、受控 outDir、manifest、`envDir` 和显式 base              |
| `android/app/build.gradle.kts` | 删除当前目录存在判断、源码树 Sync 与 `preBuild` 挂接；接入 generated assets |
| `android/buildSrc/`            | 前端构建、校验、暂存和包验证任务类型                                        |
| `android/app/.gitignore`       | 删除 legacy `src/main/assets/frontend` 忽略规则                             |
| `android/docs/README.md`       | 更新前端资源构建说明                                                        |
| `docs/code-map/android.md`     | 实施后记录新任务图和生成目录                                                |
| CI 配置                        | 提供可用的 Node/pnpm、frozen install、包级验证与缓存策略                    |

本方案不要求修改 `FrontendPathHandler.ASSET_ROOT`、受信任 origin 或 Shell Bridge 协议。

## 15. 分阶段实施

### 阶段一：先修复正确性

1. 增加 Android 专用 Vite mode并隔离环境。
2. Gradle 自动运行当前前端构建。
3. 输出迁移到 `android/app/build/`。
4. 使用 generated assets 接入 variant。
5. 删除并禁止 legacy `src/main/assets/frontend`。
6. 缺少工具或构建失败时严格失败。

完成标志：不存在“Android 构建成功但使用旧前端”的路径。

### 阶段二：增加完整校验

1. 开启 Vite manifest。
2. 增加产物引用完整性校验。
3. 增加 APK/AAB 包级验证。
4. CI 接入 frozen install、configuration cache 和离线启动 smoke。

完成标志：能证明最终包携带了本次构建且完整的前端资源。

### 阶段三：优化性能

1. 验证不同平台上的产物确定性。
2. 将任务标记为可缓存并接入 CI build cache。
3. 根据实际数据决定 Debug/Release 是否共享前端生成任务。
4. 记录任务耗时与 bundle 体积趋势。

完成标志：正确性不降低的前提下，未变更构建不重复执行前端编译。

当前实施结果：阶段一已完成；阶段二的目录校验、APK/AAB 包级校验和 API 33 真机离线 smoke 已完成，CI frozen install 待后续环境执行；阶段三已完成本机 `UP-TO-DATE` 和 configuration cache 验证，跨系统确定性与共享远程缓存仍保持禁用。

## 16. 验收清单

### 16.1 构建正确性

- [x] 干净产物状态下不存在 `frontend/dist` 和 legacy Android frontend assets 时，准备依赖后 `assembleDebug` 成功。
- [x] 修改任意 `frontend/src` 文件会触发前端重建。
- [x] 连续两次无变更构建中，前端生成任务为 `UP-TO-DATE`。
- [x] 人为制造 TypeScript 错误时 Android 构建失败。
- [x] 删除 Android 生成目录后不会从任何旧目录回退。
- [x] `gradlew clean` 会清理所有 Android 前端生成资源。
- [x] configuration cache 首次存储、再次复用均成功。

### 16.2 环境与可复现性

- [x] Android 构建直接使用本机 Node/pnpm，且不会自动下载或切换版本。
- [ ] CI 使用 frozen lockfile，构建期间锁文件不发生变化。
- [x] 本机 `.env.local` 与 Android mode env 文件中的值不会进入 Android bundle。
- [x] 父进程 `VITE_*` 变量不会影响 Android bundle。
- [ ] Windows 与 CI 目标系统使用相同输入能产生相同文件清单；启用远程缓存前进一步比较内容摘要。

### 16.3 产物完整性

- [x] Vite manifest 中的全部入口、CSS、静态资源和 dynamic import 都存在。
- [x] `index.html` 的本地引用全部可解析。
- [x] Release 输出不包含 source map、Vite dev client 或 HMR 地址。
- [x] APK/AAB 同时包含 `assets/shell/android-transport.js` 与完整 `assets/frontend`。
- [x] 包内 manifest 摘要与已验证暂存产物一致。

### 16.4 运行 smoke

- [x] Android 设备离线且 API 不可用时，仍可加载包内前端。
- [x] 页面从 `https://winestock.internal/` 加载，根绝对资源路径正常。
- [x] Shell Bridge 在受信任 origin 正常注入并收到 `frontendReady`；运行设置与本地服务生命周期调用可用。
- [x] 前端入口、懒加载路由、CSS、SVG 和字体无 404。
- [x] WebView 控制台无资源加载 error；连接明文 HTTP 远端时仅有当前策略预期的 mixed-content warning。

### 16.5 已执行验证记录

- 本机 pnpm 直接执行 `vue-tsc -b && vite build --mode android` 成功。
- `assembleDebug`、`assembleRelease` 和 `bundleRelease` 成功，且对应 APK/AAB 包级前端验证成功。
- 重复构建中 `buildAndroidFrontend`、资源校验、暂存和包级验证均命中 `UP-TO-DATE`，configuration cache 成功复用。
- 临时移开 `frontend/dist` 并执行 `:app:clean` 后，`assembleDebug` 仍从当前源码重新生成前端并成功打包。
- 故障注入验证了 legacy assets、manifest 缺失、manifest 引用缺失、依赖未准备和 TypeScript 错误都会中止构建。
- 受控哨兵验证了父进程 `VITE_*` 与 `.env.android.local` 不进入 Android 产物；现有 `.env.local` 内容未被读取或修改。
- 2026-07-23 在 Xiaomi `M2012K11AC`、Android 13 / API 33、ARM64 真机上通过 `:app:installDebug`
  安装当前工作树 Debug APK；断开 Wi-Fi、force-stop 后冷启动仍能显示“暂时无法连接服务”，包内前端、
  “本机运行设置”和“重新连接”入口均可操作，没有白屏、404、Uncaught、FATAL 或 APK 资源加载失败。
- 恢复网络后，打包前端可连接 `http://192.168.10.183:17890`，健康检查、登录和 dashboard 加载成功；
  WebView 对该明文 HTTP 地址记录 mixed-content warning，属于当前显式放行策略下的预期提示，不应记为
  “控制台完全无 warning”。
- 剩余覆盖项为 CI frozen install 和跨操作系统产物文件清单/摘要比较。

## 17. 不采用的方案

### 17.1 继续手工 `pnpm build` 后复制

无法建立源码到 APK 的强依赖，仍会出现忘记构建、忘记同步和复制旧目录。

### 17.2 `dist` 缺失时继续跳过

这会把“缺少必要输入”错误地解释为“可以使用旧输出”，是当前最需要移除的路径。

### 17.3 把生成物继续写入 `src/main/assets`

源码树不是 Gradle 生成目录，`clean`、IDE、缓存和来源优先级都会变得不可靠。

### 17.4 提交前端 dist

会产生大量 hash 文件变更和合并冲突，也不能证明提交的 dist 与当前源码、锁文件和工具链一致。

### 17.5 每次 assemble 自动执行 `pnpm install`

会引入隐式网络、显著延迟和不稳定失败。依赖准备与编译应分别建模。

### 17.6 用时间戳判断新旧

时间戳会受复制、解压、Git checkout、时区和文件系统影响；Gradle 输入快照与任务依赖才是正确模型。

### 17.7 让 Axum 服务前端

违反 WineStock 既定平台所有权：Android shell 负责资源打包，Axum 只拥有 HTTP 业务服务。

## 18. 参考依据

- Gradle 当前文档：lazy configuration、任务输入/输出和 build cache。
  - <https://docs.gradle.org/current/userguide/lazy_configuration.html>
  - <https://docs.gradle.org/current/userguide/bp_tasks.html>
  - <https://docs.gradle.org/current/userguide/build_cache.html>
- Vite 当前文档：build options、manifest、环境变量与 mode 加载顺序。
  - <https://vite.dev/config/build-options.html>
  - <https://vite.dev/guide/backend-integration.html>
  - <https://vite.dev/guide/env-and-mode.html>
- pnpm 当前文档：固定 package manager 与 frozen lockfile 安装。
  - <https://pnpm.io/installation>
  - <https://pnpm.io/cli/install>
