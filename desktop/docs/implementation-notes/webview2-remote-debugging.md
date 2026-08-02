# WebView2 远程调试整理

## 结论

Tauri Windows 使用 Microsoft WebView2。WebView2 基于 Chromium，因此支持 Chrome DevTools
Protocol（CDP）远程调试。WineStock 只在 Debug 构建开启 CDP，Release 构建禁止通过环境变量或
其它外部参数开启。

当前实现位于 [`desktop/src/webview_debug.rs`](../../src/webview_debug.rs)，由 desktop Rust Shell
在创建 Tauri 主窗口前调用。WebView2 必须在创建 CoreWebView2 环境前收到浏览器参数，前端页面
或窗口创建后的命令都不能可靠地补充这个配置。

## 构建策略

| 构建类型                      | CDP 状态 | 监听地址    | 端口配置                                                         |
| ----------------------------- | -------- | ----------- | ---------------------------------------------------------------- |
| Debug / `tauri dev`           | 开启     | `127.0.0.1` | 默认 `9222`，可用 `WINESTOCK_WEBVIEW2_CDP_PORT` 覆盖             |
| Debug / `tauri build --debug` | 开启     | `127.0.0.1` | 默认 `9222`，可用 `WINESTOCK_WEBVIEW2_CDP_PORT` 覆盖             |
| Release / `tauri build`       | 禁止     | 无          | 忽略调试端口配置，并清理 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` |

Debug 构建由 Rust 生成固定的 WebView2 参数：

```text
--remote-debugging-address=127.0.0.1
--remote-debugging-port=9222
```

调用进程原本设置的 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` 不会被直接透传。Debug 下它会被
WineStock 自己生成的参数覆盖，Release 下会被清理。这避免了任意 Chromium 参数进入桌面壳，
也使 Debug/Release 行为可预测。

## 启动与端口覆盖

默认启动：

```powershell
cd frontend
pnpm desktop:dev
```

端口 `9222` 被占用时，在启动 Tauri 前指定项目自己的环境变量：

```powershell
$env:WINESTOCK_WEBVIEW2_CDP_PORT = "9223"
pnpm desktop:dev
Remove-Item Env:WINESTOCK_WEBVIEW2_CDP_PORT
```

端口必须是 `1` 到 `65535` 的整数。无效值会回退到 `9222`；Release 构建不会读取该变量。

## MCP、Chrome DevTools 与 WebDriver 连接

启动 Debug 桌面壳后，先检查 CDP 服务：

```text
http://127.0.0.1:9222/json/version
http://127.0.0.1:9222/json/list
```

其中 `/json/version` 用于确认调试端点和 WebSocket 地址，`/json/list` 用于查看当前 WebView
页面目标。Chrome DevTools、Edge DevTools、支持 CDP 的 MCP 工具和 WebDriver 都可以使用这个
端点；WebDriver 连接时使用 `127.0.0.1:9222` 作为 debugger address。

这不是 WineStock 业务 API，也不是 Shell Bridge；它只用于开发者查看/调试 WebView 页面。

## 安全边界

- 只绑定 `127.0.0.1`，不允许为了让其它设备连接而改成 `0.0.0.0`。
- CDP 端点没有 WineStock 登录鉴权；能访问端口的进程可以检查和控制 WebView 页面。
- 远程调试只服务本机开发和 MCP 调试，不用于测试环境对外开放，也不进入安装包运行策略。
- Release 启动前清理 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`，防止宿主环境残留参数意外开启调试。
- 不把端口配置暴露为前端设置项；它是 desktop 开发启动参数，不属于 Shell Bridge 契约。

## 验收

Windows Debug 验收至少包括：

1. 默认启动后 `127.0.0.1:9222/json/version` 可访问。
2. Chrome DevTools 或 MCP 能列出 WineStock 主窗口页面并执行基础页面检查。
3. 设置 `WINESTOCK_WEBVIEW2_CDP_PORT=9223` 后，`9222` 不再是端点，`9223` 可访问。
4. 设置任意 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` 后，Debug 仍只使用 WineStock 生成的本机 CDP 参数。
5. Release 启动后 `9222` 不可访问，且外部 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` 不会改变这一结果。

Rust 侧的端口解析测试使用：

```powershell
cargo test -p winestock-desktop webview_debug
```
