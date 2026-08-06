# Desktop 与 Android Shell 更新检测和安装实施报告

## 结论

更新检测由三个平台 Shell 负责；Desktop 与 Android 安装各自的制品，Server 只报告可用制品。共享前端不直接请求更新清单、不下载安装包，也不访问更新域名；前端只通过现有 Shell Bridge 获取检查结果并展示更新状态。

这样可以避免 WebView 中的跨域限制，也能把文件下载、临时文件、安装权限和平台生命周期留在正确的所有权边界内。

本方案不接入 Tauri updater，不使用 Tauri updater 的 manifest 或签名协议。Desktop 和 Android 各自实现对应平台的检查与安装流程。

## 所有权边界

### Desktop Shell

Desktop 负责：

- 请求 Desktop 更新清单；
- 使用当前 Tauri 包版本比较远端版本；
- 校验 HTTPS、版本关系、文件类型、文件大小和 SHA-256；
- 将 Windows 安装包下载到应用缓存或临时目录；
- 启动安装器并请求当前应用优雅退出；
- 将检查结果、下载失败和安装失败转换为稳定的 Shell Bridge 错误。

Desktop 不负责：

- 修改 core HTTP API；
- 让 Axum 提供更新清单或安装包；
- 让前端直接使用 `fetch` 下载安装包；
- 在运行中的进程内覆盖自身可执行文件。

### Android Shell

Android 负责：

- 请求 Android 更新清单；
- 使用当前 `versionName` 比较远端 `version`；
- 校验 HTTPS、版本关系、APK 文件类型、文件大小和 SHA-256；
- 将 APK 保存到应用私有缓存目录；
- 通过 `FileProvider` 提供受控 `content://` URI；
- 调起系统 APK 安装器并处理安装权限缺失；
- 将检查结果、下载失败、权限缺失和安装失败转换为稳定的 Shell Bridge 错误。

Android 不负责：

- 通过 WebView 页面直接下载 APK；
- 使用 `file://` URI 暴露应用文件；
- 复制一套前端设置 Activity 或原生更新 Dialog；
- 在应用内部绕过系统安装确认。

### Server Shell

Server 负责：

- 通过 `winestock-server --check-update` 请求 GitHub 最新正式 Release；
- 使用当前 Cargo 包版本比较 Release tag 版本；
- 发现更新时输出 GitHub Release 页面地址，交由用户手动下载和部署。

Server 不负责：

- 在常规服务启动期间访问更新服务；
- 在运行中的服务进程内覆盖自身文件；
- 替代部署系统下载或安装 Server ZIP。

### Frontend

前端只负责：

- 在应用启动后后台触发检查，或响应偏好设置中的手动检查；
- 显示当前版本、可用版本、更新说明和检查失败状态；
- 检查失败时显示面向用户的可恢复提示，不静默吞掉错误；
- 在用户明确点击后调用 Shell Bridge 的安装方法；
- 在安装即将开始时保留当前页面状态，等待 Shell 重启或系统安装器接管。

前端不拥有更新清单 URL、平台下载 URL、APK 文件路径或 Desktop 临时文件路径。

## 更新清单

Desktop 和 Android 使用统一清单；Server 独立检查 GitHub Release。清单不需要包含 `mandatory`，当前产品只支持普通更新提示，不支持强制升级。

### 当前清单地址

`https://api.ikuns.top/WineRealm/file/winestock/winestock.json`

该地址由三个 Shell 直接请求。共享前端不得直接访问，也不得把清单地址作为前端可编辑配置或业务 API 地址处理。正式实现中应继续限制请求使用 HTTPS、设置超时、校验响应内容类型和清单结构，并在 Shell 内把网络异常转换为稳定错误码。

### 统一清单

```json
{
  "version": "0.1.1",
  "baseUrl": "https://tapan.top/file/winestock",
  "notes": "修复若干问题并改进启动稳定性。",
  "desktop": {
    "file": "WineStock-0.1.1-setup.exe",
    "sha256": "十六进制 SHA-256"
  },
  "android": {
    "file": "WineStock-0.1.1-release.apk",
    "sha256": "十六进制 SHA-256"
  }
}
```

字段约束：

- `version` 是面向用户和更新比较的语义版本；不能使用字符串字典序比较；
- `baseUrl` 必须是不含凭据、查询参数或片段的 HTTPS 地址；
- `desktop`、`android` 都必须提供相对 `file` 与 SHA-256；Shell 只下载自己的制品；
- `file` 不得是绝对路径、不得包含目录穿越、查询参数或片段；下载地址固定为 `baseUrl + "/" + file`；
- `sha256` 用于发现传输损坏或文件不完整；
- `notes` 可以为空，但清单结构必须完整；
- 清单缺失、JSON 无法解析、版本格式无效或资产字段无效时，按检查失败处理，不展示部分结果。

SHA-256 不是发布签名。它只能证明下载文件与清单中的摘要一致，不能防止清单服务器和安装包同时被替换。本阶段接受这个安全等级；后续若需要抵抗发布源被入侵，再单独引入签名方案。

清单 URL 应由各 Shell 的发布配置提供，不应把开发环境、测试环境和生产环境地址混在前端构建产物中。

## 版本语义

Desktop、Android 和 Server 的检查都只比较清单中的 `version` 与当前 Shell 版本：

- 相等：没有更新；
- 远端更高：有更新；
- 远端更低：忽略降级；
- 任一版本无法解析：检查失败；
- `0.1.10` 必须大于 `0.1.9`，不能按普通字符串比较。

Android 更新清单不需要增加 `versionCode` 字段。`versionCode` 仍然必须存在于 APK 自身的 Android manifest 中，并且每次正式发布必须递增，这是 Android Package Manager 的安装要求，不属于前端更新比较字段。

根 Cargo 工作区的 `[workspace.package].version` 是三个 Shell 的唯一发布版本来源：Desktop Tauri 在未指定配置版本时继承 Cargo 包版本，Server 使用同一 Cargo 包版本；Android 在 Gradle 构建时读取该值作为 `versionName`，并按 `major * 1_000_000 + minor * 1_000 + patch` 派生内部 `versionCode`。更新 APK 仍必须保持相同的 `applicationId` 和正式签名证书，否则系统会拒绝覆盖安装。

## Shell Bridge 建议

在现有可选平台扩展中增加两个具名能力：

```ts
interface AppUpdateShellBridgeExtension {
  checkForUpdate(): Promise<AppUpdateCheckResult>;
  installUpdate(version: string): Promise<void>;
}

interface AppUpdateCheckResult {
  currentVersion: string;
  latestVersion?: string;
  notes?: string;
  publishedAt?: string;
}
```

安装方法只接收版本号。Shell 必须在安装前重新请求清单并确认版本仍然匹配，不能信任前端传入的下载地址，也不能直接安装上一次检查缓存中的过期地址。

建议的失败码：

- `update_check_unavailable`：清单请求失败、超时或服务器返回无效状态；
- `update_manifest_invalid`：清单结构或版本字段无效；
- `update_not_available`：安装请求对应的版本已不再可用；
- `update_download_failed`：安装包下载失败或超过大小限制；
- `update_integrity_failed`：SHA-256 校验失败；
- `update_install_permission_required`：Android 尚未允许当前应用安装未知来源应用；
- `update_install_failed`：平台安装器无法启动或安装过程启动失败。

Server 通过 `winestock-server --check-update` 输出 GitHub Release 页面地址，不在常规服务启动时联网，也不执行进程内自更新。

Shell 不应把原始网络异常、HTTP 响应体、文件路径或堆栈直接传给前端。所有失败都应映射为稳定错误码和安全的用户提示文案。

## 检测失败的前端提示

检测失败必须进入共享前端的 Notice/状态反馈，不得只写入原生日志或静默忽略：

- 用户在偏好设置中手动检查时：显示错误 Notice，说明“暂时无法检查更新”，并提供“重试”操作；
- 应用启动后的后台检查失败时：显示非阻塞的警告 Notice，不阻止登录、业务页面或本地服务启动；
- 网络超时、服务不可达：提示检查网络或稍后重试；
- 清单格式错误或版本无效：提示更新服务暂时不可用，不展示原始 JSON；
- SHA-256 校验失败：提示更新文件校验失败，禁止继续安装；
- Android 未允许安装未知来源：提示需要在系统设置中允许当前应用安装，并提供进入设置的恢复操作；
- 用户已经手动检查并看到失败后，后台检查不应在同一会话内重复堆叠相同 Notice；
- 检查失败不应清除当前版本、登录会话、运行配置或业务页面状态。

前端可将 Shell Bridge 错误映射为本地化文案，但不能根据异常字符串猜测类别。安装失败同样必须显示错误 Notice，并保留“重试检查”入口；只有当前 Shell 已明确启动系统安装器时，才允许提示用户转到系统安装界面继续处理。

## Desktop 安装流程

1. Shell 使用当前 Tauri 包版本请求 Desktop 清单。
2. 校验版本、HTTPS 域名、`.exe` 扩展名、响应大小和 SHA-256。
3. 下载到应用缓存目录或系统临时目录，文件名使用版本生成，不能使用远端 URL 的原始文件名作为路径。
4. 使用平台进程 API 启动安装器。
5. 请求当前 Tauri 进程优雅退出，确保本地 Axum 服务释放端口和数据库句柄。
6. 安装器完成文件替换，用户重新启动新版本。

安装器启动后不能依赖旧 WebView 继续工作。安装失败应保留旧版本和已下载文件的可诊断状态，不删除用户数据。

## Android 权限与安装流程

### 已有网络权限

Android 当前 manifest 已声明 `android.permission.INTERNET`，可直接用于请求更新清单和下载 APK。更新下载应在 Shell 的后台线程或协程执行，不能阻塞 WebView/UI 线程。

### 安装未知来源权限

从应用内安装 APK 需要在 manifest 声明：

```xml
<uses-permission android:name="android.permission.REQUEST_INSTALL_PACKAGES" />
```

该权限不是普通危险权限，不能用 `requestPermissions()` 像相机权限一样直接弹出授权框。安装前应：

1. 使用 `packageManager.canRequestPackageInstalls()` 检查当前应用是否被允许安装未知来源应用；
2. 未授权时通过 `Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES` 跳转到当前应用的系统设置页；
3. 用户返回 Activity 后重新检查授权状态；
4. 只有授权成功后，才启动 APK 安装 Intent；
5. 用户拒绝或设备不支持该设置页时，前端显示可恢复的提示，不认为更新检查失败。

这是一次系统设置授权，不应在 Shell 内自绘 Android 设置 Dialog。具体解释和恢复入口仍由共享前端呈现。

### FileProvider

下载完成后不能把缓存文件转换为 `file://` URI。应：

- 在 manifest 注册 `androidx.core.content.FileProvider`；
- 使用 `android:exported="false"` 和 `android:grantUriPermissions="true"`；
- 通过 `res/xml/file_paths.xml` 只暴露专用更新缓存目录；
- 生成 `content://` URI；
- 启动安装 Intent 时附加 `FLAG_GRANT_READ_URI_PERMISSION` 和必要的临时授权；
- MIME 类型使用 `application/vnd.android.package-archive`；
- 不向 WebView 或外部任意应用暴露整个缓存根目录。

推荐安装 Intent：

```text
ACTION_VIEW
data = content://<applicationId>.fileprovider/<update-cache-file>
type = application/vnd.android.package-archive
FLAG_GRANT_READ_URI_PERMISSION
FLAG_ACTIVITY_NEW_TASK（从非 Activity Shell 上下文启动时）
```

系统安装器可能要求用户确认安装，即使已经拥有 `REQUEST_INSTALL_PACKAGES`。Shell 只负责启动系统流程，不应假设调用返回就代表安装成功。

### Android 数据和签名约束

- 同一 `applicationId` 才能执行覆盖安装；
- 新 APK 必须使用与已安装 APK 相同的签名证书；
- 新 APK 的 `versionCode` 必须大于已安装版本；
- 数据库、运行配置和认证数据应由 Android 系统保留，更新流程不得清理应用数据；
- Debug APK 不应作为正式更新包发布。

## 前端交互

更新入口放在现有偏好设置中，复用当前 Dialog、Notice 和异步状态规则：

- 启动后后台检查一次，检查失败不阻断业务；
- 偏好设置提供“检查更新”；
- 偏好设置提供“启动时自动检测更新”开关，默认开启；关闭后只跳过启动检查，不影响手动检查；
- 启动检测和偏好设置检测发现更新时，都打开共享的应用更新 Dialog；偏好设置入口使用嵌套层级显示在当前偏好 Dialog 上方；
- 无更新时显示当前版本和检查结果；
- 有更新时显示最新版本、更新说明和“立即安装”；
- 检测失败时显示前端 Notice，启动检测使用非阻塞警告，偏好设置检测使用错误提示；
- 安装按钮在 Shell 返回失败前保持稳定禁用状态，避免重复下载；
- Android 权限缺失时显示“去系统设置允许安装”，用户返回后可以重试；
- Web fallback 不显示平台安装入口。

前端不展示临时文件路径、服务器错误堆栈或内部下载 URL。所有平台差异通过结构化状态和稳定错误码表达。

## 验收矩阵

### 纯逻辑测试

- 版本相等、远端更高、远端更低；
- `0.1.10` 与 `0.1.9` 的正确排序；
- 版本格式非法；
- 清单缺字段、字段类型错误和 URL 非 HTTPS；
- SHA-256 正确、错误和下载截断；
- 超过文件大小限制；
- 安装请求版本过期或清单已变化。

### Desktop smoke

- 正常无更新；
- 发现更新并下载 NSIS 安装器；
- 安装器启动后当前进程和本地 Axum 优雅退出；
- 清单请求失败不影响应用启动；
- 清单请求失败时前端出现可重试提示，不只记录日志；
- 下载失败、摘要错误和安装器启动失败保留旧版本可运行；
- 远端 URL 不能被前端参数替换。

### Android smoke

- API 28+ 真机检查无更新和发现更新；
- 未允许未知来源时跳转当前应用设置页；
- 返回后授权成功，系统安装器正常打开；
- 用户拒绝授权后可回到前端重试；
- 更新清单请求失败时前端出现可重试提示，不阻塞本地 core 和业务页面；
- `content://` URI 可被系统安装器读取，不能出现 `FileUriExposedException`；
- 相同签名、递增 `versionCode` 的 APK 可以覆盖安装并保留本地数据；
- 签名不一致、`versionCode` 未递增和损坏 APK 被系统或 Shell 正确拒绝。

## 实施顺序

1. 固定 Desktop/Android 各自的清单地址和字段契约。
2. 在 Shell Bridge 增加可选更新扩展及稳定错误码。
3. 实现 Desktop 检查、下载和安装器启动。
4. 实现 Android 清单检查、APK 下载、未知来源授权和 FileProvider 安装流程。
5. 在共享前端偏好设置接入统一状态展示。
6. 从统一版本来源派生 Android `versionName` 与递增的 `versionCode`，并由 CI 生成 Desktop/Android 清单；Server 发布包由 GitHub Release 托管。
7. 执行纯逻辑测试、Desktop 安装包 smoke 和 Android 真机安装 smoke。

## 当前未包含

- Tauri updater 插件；
- Tauri updater 签名和公钥管理；
- 强制升级；
- 后台静默安装；
- Google Play 内购/商店更新；
- server shell 进程内自更新；
- core HTTP 更新 API。
