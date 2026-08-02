# Android 启动失败文案整改方案

## 问题

当前兼容性页的标题固定为“需要更新系统 WebView”，部分错误正文也直接要求更新 WebView。这样会把以下不同来源混成一个结论：

- Android 没有可用 WebView provider；
- provider 版本过低或无法读取版本；
- WebView 缺少 WineStock 所需接口；
- WineStock 自身 Shell Bridge 安装、契约校验或首屏握手失败。

最后一类很可能是应用版本、前端资源或 Shell Bridge 实现问题，不应默认归因于系统 WebView。

## 整改目标

1. 标题和正文必须由 `WebViewIncompatibilityReason` 驱动，不再使用固定“需要更新系统 WebView”。
2. 只有 `VERSION_TOO_OLD` 明确建议更新 WebView；其它原因说明实际检测结论和可能责任边界。
3. 应用自身失败使用“WineStock 加载失败”语义，不把用户引导到系统更新。
4. 保留当前 provider、版本和最低要求等诊断信息，便于反馈问题和定位设备差异。
5. “重新检测”继续保持原生恢复页的单一恢复入口；不自动跳转应用商店或系统设置。

## 建议文案分类

| 原因                          | 建议标题            | 建议正文方向                                                                              |
|-----------------------------|-----------------|-------------------------------------------------------------------------------------|
| `PROVIDER_UNAVAILABLE`      | 无法使用系统 WebView  | 系统未提供可用的 WebView，可能是系统组件被停用或设备环境异常。请确认 Android System WebView 或 Chrome 已启用后重试。      |
| `VERSION_UNREADABLE`        | 无法确认 WebView 状态 | WineStock 无法读取系统 WebView 的版本信息，可能是系统组件异常或当前设备实现差异。请重新检测；若持续失败，请反馈诊断信息。              |
| `VERSION_TOO_OLD`           | 系统 WebView 版本过低 | 当前 WebView 版本低于 WineStock 的最低要求。请更新 Android System WebView 或 Chrome 后重试。            |
| `REQUIRED_FEATURES_MISSING` | WebView 能力不兼容   | 当前 WebView 缺少 WineStock 需要的安全通信接口，可能与 WebView 版本、系统实现或应用兼容性有关。请先更新后重试；若仍失败，请反馈诊断信息。 |
| `SHELL_BRIDGE_UNAVAILABLE`  | WineStock 加载失败  | WineStock 与系统 WebView 的桥接未能启动，可能是应用资源、应用版本或系统兼容性问题。请重新打开应用；若仍失败，请更新应用或反馈诊断信息。       |

对应的用户可见错误码为：

| 原因 | 错误代码 |
|---|---|
| `PROVIDER_UNAVAILABLE` | `WEBVIEW_PROVIDER_UNAVAILABLE` |
| `VERSION_UNREADABLE` | `WEBVIEW_VERSION_UNREADABLE` |
| `VERSION_TOO_OLD` | `WEBVIEW_VERSION_TOO_OLD` |
| `REQUIRED_FEATURES_MISSING` | `WEBVIEW_REQUIRED_FEATURES_MISSING` |
| Shell Bridge 不可用 | `SHELL_BRIDGE_UNAVAILABLE` |

## 实施结果

1. `WebViewCompatibilityScreen` 已按原因切换标题，不再固定显示“需要更新系统 WebView”。
2. 五类原因的标题、正文、检测结果标签和诊断区域已拆成独立 Android string resource。
3. 兼容页已增加检测结果标签；Shell Bridge 失败时把“最低要求”区域替换为“失败阶段”，避免暗示系统版本必然有问题。
4. `SHELL_BRIDGE_UNAVAILABLE` 已使用应用加载失败语义，并保留应用资源、应用版本和系统兼容性的多因归因。
5. provider、版本和缺失能力仍由现有日志记录，内部异常不展示给用户。
6. 每条兼容性页正文末尾追加稳定的 `错误代码：...`；WebView 原因使用 `WEBVIEW_*` 码，Shell Bridge 使用
   `SHELL_BRIDGE_*` 或 `FRONTEND_LOAD_TIMEOUT`。未知前端值降级为 `SHELL_BRIDGE_UNAVAILABLE`，不显示原始输入。
7. 已增加 JVM 文案映射和错误码白名单测试；下一步使用以下 Debug 参数进行真机验收：
   - `FORCE_WEBVIEW_BLOCK` 验证系统 WebView 版本过低路径；
   - `FORCE_SHELL_BRIDGE_BLOCK` 验证应用桥安装失败路径；
   - `FORCE_SHELL_BRIDGE_HANDSHAKE_BLOCK` 验证应用桥握手失败路径。

## 验收标准

- 版本过低时，用户能明确知道需要更新系统 WebView。
- Bridge 安装或握手失败时，用户不会被告知“必须更新 WebView”；页面应明确这是 WineStock 加载失败，并保留重试/反馈方向。
- 页面正文末尾包含稳定错误码，且未知 Bridge 输入不会出现在页面中。
- provider 不存在、版本不可读和缺失能力三类提示互不混淆。
- Debug 注入参数覆盖三条路径，WebView 销毁、原生阻断页和返回重试均符合预期。
- Release APK 不响应 Debug 注入参数。

## 当前验证

- `:app:testDebugUnitTest` 已通过，包含五类原因映射和失败阶段标签测试。
- 仍需在代表性真机上逐项检查实际换行、深浅色资源、WebView provider 信息和三个 Debug 注入参数。
