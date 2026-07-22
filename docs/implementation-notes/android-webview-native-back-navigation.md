# WineStock Android WebView 原生返回键协商实施方案

> 文档状态：已实施，真实设备矩阵待统一上机验证<br>
> 涉及组件：`android`、`frontend`、根项目文档<br>
> 编制日期：2026-07-23<br>
> 适用范围：Android API 26 及以上，当前 `targetSdk = 36`

> 实施记录：Android broker、页面代次、400ms 超时、Bridge 事件/应答、Activity fallback、前端
> priority/LIFO registry、通用浮层与主要页面接入、出库异步离开确认均已落地。本文第 2 节保留的
> “当前实现”证据是实施前基线；完成定义中的真实设备手势/三键导航验收将在有在线设备时统一执行。

## 1. 结论

实施前的 Android 返回事件**不会透传给前端**。

实施前的 `MainActivity.installBackNavigation()` 只执行以下两级逻辑：

```text
WebView.canGoBack() == true
  -> WebView.goBack()

WebView.canGoBack() == false
  -> Activity / 系统返回
```

这能处理 Vue Router 写入 WebView history 的普通路由返回，但无法感知只存在于前端内存中的临时界面状态，例如：

- 通用 `ModalDialog` 及嵌套 Dialog；
- 图片全屏预览；
- 移动导航 Drawer、库位分组 Drawer、入库明细 Drawer；
- 账户 Popover、选择器 Listbox、图片来源 Popover；
- 入库、出库工作台的页面内步骤；
- 未保存草稿触发的前端离开确认。

推荐把返回流程改为“**Android 发起请求，前端明确结算，超时后 Android 安全回退**”：

```text
Android 返回提交
  -> Shell Bridge 发送 nativeBackRequested(requestId, canGoBack)
  -> 前端按 LIFO 与优先级处理最上层临时状态
  -> 前端 resolveNativeBack(requestId, handled, reason)
  -> handled = true：Android 不再执行其它返回
  -> handled = false / 超时 / 页面未就绪：
       WebView.canGoBack() ? WebView.goBack() : Activity 系统返回
```

第一版建议采用以下固定决策：

| 决策项            | 推荐值                                                                |
| ----------------- | --------------------------------------------------------------------- |
| 通信方式          | 复用现有 origin-restricted WebMessage 通道                            |
| Shell Bridge 版本 | 保持 v1，作为 `capabilities.nativeBack` 控制的可选扩展                |
| Native 超时       | `400ms`                                                               |
| 请求并发          | 同一时刻最多一个；等待期间的重复返回直接消费，不排队                  |
| 前端顺序          | 临时浮层 → Dialog → Drawer/Popover → 页面内步骤 → Vue Router → 未处理 |
| 路由确认          | 调用 `router.back()` 后立即报告已处理，不等待导航守卫或动画结束       |
| Busy Dialog       | 消费返回但不关闭，防止提交过程中退出或露出下层页面                    |
| Predictive back   | 第一版只处理手势提交结果，不向 Web 传递手势进度                       |
| 失败回退          | 重新检查 `WebView.canGoBack()`，否则交回 Activity                     |

该方案与 `docs/shell-bridge.md` 已有约束一致：原生返回键先允许前端关闭 Dialog、Drawer 或执行路由返回，前端未处理或超时后 Activity 才执行系统返回。

## 2. 当前实现与问题证据

### 2.1 Android 当前行为

`android/app/src/main/java/winestock/xiaowine/cc/MainActivity.kt` 当前注册了生命周期感知的 `OnBackPressedCallback`，但回调只判断 WebView history：

```kotlin
if (binding.webView.canGoBack()) {
    binding.webView.goBack()
} else {
    isEnabled = false
    onBackPressedDispatcher.onBackPressed()
}
```

因此 Android 不知道以下状态是否打开：

- `.modal-layer`；
- `PreviewImage.viewerOpen`；
- `AppShell.navOpen`；
- `LocationsPage.groupPanelOpen`；
- `InboundDraftPage.selectedLineId`；
- `SelectControl.open`；
- 其它页面内 ref 状态。

### 2.2 Shell Bridge 当前缺口

现有 Shell Bridge v1 已具备适合承载返回协商的基础：

- JS → Native：`{ type: "call", id, method, params }`；
- Native → JS：`{ type: "reply", id, ok, result?, error? }`；
- Native → JS：`{ type: "event", event, payload? }`；
- `WebMessageListener` 只允许受信任 origin 的主框架；
- `JavaScriptReplyProxy` 已用于向当前页面推送事件；
- `RuntimeSnapshot.capabilities.nativeBack` 已存在。

缺少的是：

1. `nativeBackRequested` 事件；
2. 前端订阅方法；
3. `resolveNativeBack` 应答方法；
4. Android pending request、超时和重复结算状态机；
5. 前端统一的返回 handler registry。

实施前 `RuntimeSnapshotFactory.kt` 把 `nativeBack` 固定为 `false`，这与当时能力一致；协议完整实现前不得提前改为 `true`。

### 2.3 Vue Router 只能覆盖一部分场景

前端使用 `createWebHashHistory()`。路由跳转会进入浏览器/WebView history，因此 `WebView.goBack()`通常能够回到上一条路由。

但以下变化不会自动建立路由 history：

- 打开或关闭 Dialog；
- 打开 Drawer、Popover 或 Select listbox；
- 打开图片预览；
- 切换工作台内部步骤；
- 打开页面内编辑面板；
- 弹出未保存草稿确认。

如果继续只依赖 `WebView.goBack()`，返回键可能在 Dialog 仍打开时直接切换路由，甚至退出 Activity。

## 3. 方案选择评估

| 方案                                      | 优点                                 | 主要问题                                                      | 结论     |
| ----------------------------------------- | ------------------------------------ | ------------------------------------------------------------- | -------- |
| 保持 `WebView.goBack()`                   | 实现最少                             | 无法关闭纯前端状态；返回语义错误                              | 不采用   |
| 给每个浮层写入 URL/history                | 系统返回天然可见                     | 污染 URL 和 history；嵌套浮层、busy 状态与草稿确认复杂        | 不采用   |
| Android `evaluateJavascript()` 调全局函数 | 表面直接                             | 建立第二套桥；异步 Promise 结算和页面重载难管理；安全边界更差 | 不采用   |
| 前端只发送“当前有浮层”状态                | Android 可决定是否消费               | 状态容易过期；Android仍不知道应关闭哪个组件；职责倒置         | 不采用   |
| WebMessage 请求—应答协议                  | 复用现有安全通道；可超时、去重和观测 | 需要两端状态机和前端统一 handler registry                     | **采用** |

本方案不增加任意 native invoke，也不让 Android 理解前端组件名称。Android 只关心“本次返回是否已被处理”，具体关闭顺序由前端拥有。

## 4. 目标与非目标

### 4.1 目标

- Android 返回键和返回手势提交后，优先关闭最上层、当前可见的前端临时状态。
- 嵌套浮层按后打开先关闭的 LIFO 顺序处理。
- 没有临时状态时，允许 Vue Router 执行应用内历史返回。
- 前端无法处理、没有 history、页面未就绪或脚本无响应时，Android 能可靠回退或退出。
- 不因连续点击、迟到应答、页面刷新或 Activity 销毁造成重复返回。
- 普通浏览器和 Vite 开发环境保持 `nativeBack = false`，不模拟 Android 系统行为。
- 保持受信任 origin、主框架、具名方法和结构化消息的现有安全边界。

### 4.2 非目标

- 不把所有前端状态都写入 URL。
- 不把 Android 返回变成通用键盘事件或 DOM `popstate` 代理。
- 不在本次实现完整 predictive-back 进度动画、取消动画或页面截图预览。
- 不让 Android 分辨 `ModalDialog`、Drawer、草稿或业务页面。
- 不等待关闭动画、网络请求或业务提交完成后才结算返回请求。
- 不同时实现 Desktop 原生返回；Desktop 保持 capability 关闭，未来复用同一前端 registry。
- 不修改 HTTP API、core、shared、数据库或鉴权契约。

## 5. 职责边界

### 5.1 Android shell 负责

- 通过 `OnBackPressedDispatcher` 接收返回提交。
- 判断当前页面和桥是否具备协商条件。
- 创建唯一 `requestId` 并发送结构化事件。
- 维护至多一个 pending request、超时任务和生命周期取消。
- 校验前端应答并保证每个请求最多结算一次。
- 前端未处理或失效时执行 WebView/Activity 安全回退。
- 记录超时、迟到应答和处理时延，不解释前端 `reason` 的业务含义。

### 5.2 frontend 负责

- 注册当前可见临时 UI 的返回 handler。
- 按优先级与打开顺序关闭最上层状态。
- 决定 busy 状态是关闭、消费还是继续向下查找。
- 在没有临时状态时调用 Vue Router 返回。
- 对每个原生请求只发送一次明确应答。
- 维护 Web fallback，不通过 User-Agent 或 Android 全局对象猜测能力。

### 5.3 Shell Bridge 负责

- 传输 `nativeBackRequested` 事件和 `resolveNativeBack` 调用。
- 限制 origin、主框架、消息结构和方法名称。
- 不暴露 Android `Activity`、`WebView` 或任意 Java/Kotlin 对象。
- 不把 UI handler registry 或 Vue Router 逻辑放进 Android shim。

## 6. 用户交互优先级

同一次返回只执行一个语义动作。推荐顺序如下：

| 优先级 | 状态类型           | 示例                                           | 返回行为                                      |
| ------ | ------------------ | ---------------------------------------------- | --------------------------------------------- |
| 500    | 瞬时子浮层         | Select listbox、图片来源 Popover、颜色选择子层 | 关闭最上层子浮层并恢复焦点                    |
| 450    | 全屏临时查看       | `PreviewImage` 全屏查看                        | 关闭图片预览                                  |
| 400    | 最上层 Dialog      | 嵌套确认、日期时间选择、业务编辑 Dialog        | 非 busy 时请求关闭；busy 时消费但保持         |
| 300    | Drawer / Popover   | 移动导航、账户菜单、库位分组、入库明细编辑     | 关闭当前面板                                  |
| 200    | 页面内导航状态     | 入库/出库工作台从“填写单据”回到“选择物品”      | 返回页面内上一步                              |
| 100    | Vue Router history | `/items` 返回 `/dashboard`                     | `router.back()`                               |
| 0      | 无法处理           | 冷启动首页且没有 history                       | `handled = false`，Android 退出或执行系统返回 |

规则：

1. 不通过遍历 DOM 猜测所有浮层；组件在打开时显式注册，关闭时显式注销。
2. 同优先级按最近激活顺序处理，确保嵌套状态后打开先关闭。
3. 一次返回不得同时关闭 Dialog 并切换路由。
4. busy Dialog、提交中确认层或不可安全关闭的事务层应返回“已消费”，不能让返回穿透到下层。
5. Notice、加载遮罩和服务不可用覆盖层默认不注册，因为它们不是用户可返回关闭的导航状态。

## 7. Shell Bridge 协议设计

### 7.1 版本策略

建议保持 `protocolVersion = 1`，把原生返回定义为已有 `capabilities.nativeBack` 的可选扩展，而不是立即升级到 v2。

原因：

- v1 已预留 `nativeBack` capability；
- 现有 call/reply/event 信封无需变化；
- Android shell 与打包前端作为同一应用发布；
- `nativeBack = false` 的旧桥仍可继续初始化。

兼容规则必须明确：

```text
capabilities.nativeBack == false
  -> onNativeBackRequested / resolveNativeBack 可以不存在，前端不得调用

capabilities.nativeBack == true
  -> 两个方法必须完整存在，否则视为 invalid_bridge_payload
```

因此 `assertCompleteShellBridge()` 不应无条件要求扩展方法；应在读取初始快照后按 capability 追加校验。若未来出现可独立升级、不能与前端同步发布的外部 shell，再把该扩展提升为 Shell Bridge v2。

### 7.2 TypeScript 逻辑契约

建议新增：

```ts
export interface NativeBackRequest {
  requestId: string;
  /** Android 在按键提交瞬间观察到的 WebView history 提示。 */
  canGoBack: boolean;
}

export type NativeBackReason =
  | "transient-overlay"
  | "image-preview"
  | "dialog"
  | "busy-dialog"
  | "drawer"
  | "popover"
  | "page-state"
  | "route-history"
  | "handler-error"
  | "unhandled";

export interface NativeBackResolution {
  requestId: string;
  handled: boolean;
  /** 仅用于诊断；Android 不得据此执行产品分支。 */
  reason: NativeBackReason;
}

export interface NativeBackResolutionAck {
  /** false 表示请求已超时、已结算、页面已重载或 requestId 未知。 */
  accepted: boolean;
}

export interface NativeBackShellBridgeExtension {
  onNativeBackRequested(
    listener: (request: NativeBackRequest) => void,
  ): Promise<StopShellSubscription>;
  resolveNativeBack(
    resolution: NativeBackResolution,
  ): Promise<NativeBackResolutionAck>;
}
```

`ShellBridge` 可用 optional extension 方法表达兼容性，运行期在 capability 为 true 时收窄为完整扩展接口。

### 7.3 Native → JS 事件

示例：

```json
{
  "type": "event",
  "event": "nativeBackRequested",
  "payload": {
    "requestId": "page-3:17",
    "canGoBack": true
  }
}
```

约束：

- `requestId` 由 Android 生成，建议为“页面代次 + 单调序号”的短字符串；
- 长度限制建议为 1～64 字符；
- `canGoBack` 只是前端决定是否尝试路由返回的提示；
- 最终 fallback 时 Android 必须重新读取 `WebView.canGoBack()`，不能使用过期值。

### 7.4 JS → Native 应答

示例：

```json
{
  "type": "call",
  "id": 42,
  "method": "resolveNativeBack",
  "params": {
    "requestId": "page-3:17",
    "handled": true,
    "reason": "dialog"
  }
}
```

原生确认：

```json
{
  "type": "reply",
  "id": 42,
  "ok": true,
  "result": {
    "accepted": true
  }
}
```

处理规则：

- 正在等待且 `requestId` 匹配：`accepted = true`；
- 迟到、重复、未知或旧页面请求：`accepted = false`，记录调试日志但不报协议错误；
- payload 类型或长度不合法：返回 `invalid_bridge_payload`；
- Android 只判断 `handled`，不根据 `reason` 决定 fallback。

### 7.5 Capability 语义

Android 只有在以下条件全部成立时才返回 `nativeBack = true`：

1. `WEB_MESSAGE_LISTENER` 可用；
2. `DOCUMENT_START_SCRIPT` 可用；
3. `ShellBridgeHost.install()` 成功；
4. 当前注入 shim 实现订阅和应答方法；
5. Native back broker 已启用。

`frontendReady` 不属于 capability，它表示当前页面代次是否已经完成订阅并可以接收事件。页面刷新期间 capability 仍可为 true，但 Activity 应暂时直接走 native fallback，直到新页面再次报告 ready。

## 8. Android 状态机设计

### 8.1 状态

```text
NotReady
  页面未 ready、无 replyProxy、正在重载或 Activity 非 resumed

Idle
  可以发起新的 native back 请求

Awaiting(requestId, deadline)
  已向当前页面发送请求，等待一次有效应答

Destroyed
  Activity / WebView 已销毁，不再执行回调或 fallback
```

状态迁移：

| 当前状态   | 事件                          | 动作                                    | 下一状态      |
| ---------- | ----------------------------- | --------------------------------------- | ------------- |
| NotReady   | 返回提交                      | 立即 native fallback                    | NotReady/结束 |
| Idle       | 返回提交且可发送              | 建立 request、发送事件、启动 400ms 超时 | Awaiting      |
| Awaiting   | 再次返回提交                  | 消费重复返回，不新建请求、不排队        | Awaiting      |
| Awaiting   | `handled = true`              | 取消超时，不再执行返回                  | Idle          |
| Awaiting   | `handled = false`             | 取消超时，执行 native fallback          | Idle/结束     |
| Awaiting   | 超时                          | 使 request 失效，执行 native fallback   | Idle/结束     |
| Awaiting   | 页面开始重载 / Activity pause | 使 request 失效，不额外 fallback        | NotReady      |
| 任意非销毁 | Activity destroy              | 清理 timeout、proxy、回调               | Destroyed     |

### 8.2 返回入口

继续使用生命周期感知注册：

```kotlin
onBackPressedDispatcher.addCallback(
    this,
    object : OnBackPressedCallback(true) {
        override fun handleOnBackPressed() {
            handleBackCommit(this)
        }
    },
)
```

AndroidX `OnBackPressedDispatcher` 继续作为唯一入口；不同时直接注册另一套平台 `OnBackInvokedDispatcher` 回调，避免优先级和生命周期冲突。

建议入口流程：

```text
handleBackCommit(callback)
  -> Activity/WebView 不可用？return
  -> broker 已 Awaiting？消费并 return
  -> frontendReady && trusted page && shellBridge.requestNativeBack(...) 成功？return
  -> performNativeFallback(callback)
```

### 8.3 Native fallback

fallback 必须在执行时重新判断当前 history：

```text
binding.webView.canGoBack()
  -> binding.webView.goBack()

否则
  -> 临时禁用当前 OnBackPressedCallback
  -> onBackPressedDispatcher.onBackPressed()
  -> 若 Activity 未结束且仍处于有效状态，再恢复 callback
```

不能直接调用已废弃的 `Activity.onBackPressed()`。

### 8.4 Pending request broker

建议新增独立类：

```text
android/app/src/main/java/winestock/xiaowine/cc/shell/NativeBackRequestBroker.kt
```

职责：

- 生成 requestId；
- 保存单个 pending request；
- 安排和取消 `Handler` timeout；
- 验证 requestId；
- 保证 success、unhandled、timeout、cancel 只结算一次；
- 向 `MainActivity` 返回最终 `handled: Boolean`；
- 支持纯 JVM 单元测试。

`ShellBridgeHost` 只负责收发信封，broker 负责协商状态，`MainActivity` 负责真实 fallback，避免把 Activity 行为写进传输分发器。

### 8.5 页面代次与 reply proxy

当前 `ShellBridgeHost` 保存最近一次 `JavaScriptReplyProxy`。实现时应增加页面代次管理：

- 受信任主页面开始加载时：增加 generation、清除旧 proxy、取消 pending request、标记 not ready；
- 新页面首次通过主框架发送消息时：保存新的 proxy；
- 新页面调用 `frontendReady` 后：允许发送 native back；
- 外部 URL 仍交系统浏览器，不向非受信任页面发送事件；
- `requestId` 携带 generation，旧页面应答不会误结算新请求。

### 8.6 超时选择

建议常量：

```kotlin
const val NATIVE_BACK_RESPONSE_TIMEOUT_MS = 400L
```

理由：

- 正常本地事件分发和 ref 更新通常应在一个事件循环内完成；
- 关闭动画不应被等待；
- 400ms 能容纳短暂主线程拥塞，又不会让“退出/返回”明显失去响应；
- 所有业务网络和持久化操作均禁止进入本次握手等待路径。

目标指标：正常处理 P95 小于 100ms，timeout fallback 在约 400～450ms 内发生。

### 8.7 连续返回

第一版不排队第二次返回：

- `Awaiting` 中再次触发时直接消费；
- 用户在第一个状态完成关闭后可再次返回；
- 避免按键抖动或手势重复提交一次关闭两层 UI；
- 不引入“最多排队一个”带来的页面重载、顺序和退出竞态。

如果真实设备验证显示 400ms 内的第二次主动点击需要保留，再单独评估单槽队列，第一版不预设复杂行为。

### 8.8 生命周期清理

以下时机必须取消 pending request 且不额外 fallback：

- 主框架页面开始导航或重载；
- Activity `onPause()` / `onStop()` 导致页面不再可交互；
- Activity `onDestroy()`；
- WebView 被销毁；
- bridge host 被替换或安装失败。

这样可以避免应用已经进入后台后，旧 timeout 再触发 Activity 返回。

## 9. 前端 handler registry 设计

### 9.1 独立模块

建议新增：

```text
frontend/src/navigation/nativeBack.ts
frontend/src/composables/useNativeBackHandler.ts
```

`shell/runtime.ts` 只提供 capability-gated 的订阅和应答包装，不保存 Dialog、Drawer 或路由优先级。

### 9.2 注册接口

建议逻辑接口：

```ts
export interface NativeBackHandlerContext {
  request: NativeBackRequest;
}

export type NativeBackHandlerResult =
  { handled: true; reason: NativeBackReason } | { handled: false };

export interface NativeBackHandlerRegistration {
  id: string;
  priority: number;
  isActive(): boolean;
  handle(
    context: NativeBackHandlerContext,
  ): NativeBackHandlerResult | Promise<NativeBackHandlerResult>;
}

export function registerNativeBackHandler(
  registration: NativeBackHandlerRegistration,
): () => void;
```

注册表规则：

- priority 降序；
- priority 相同按最近激活的序号降序；
- 关闭或卸载时必须注销；
- handler 返回未处理时才继续下一个；
- 第一个 handled 结果立即结束遍历；
- 每个请求只能产生一次最终 resolution。

### 9.3 Vue composable

`useNativeBackHandler()` 接收响应式 `active`：

```ts
useNativeBackHandler({
  id: "modal-dialog",
  active: computed(() => props.open),
  priority: NativeBackPriority.Dialog,
  handle: requestClose,
});
```

composable 应在 `active` 从 false 变 true 时重新注册，在 true 变 false 时注销，使 LIFO 顺序反映“最近打开”，而不是组件最初挂载顺序。

### 9.4 Handler 返回语义

| 情况                      | 结果                                                        |
| ------------------------- | ----------------------------------------------------------- |
| 成功关闭当前状态          | `handled = true`                                            |
| 状态已变化或不是最上层    | `handled = false`，继续查找                                 |
| busy / 提交中，不允许关闭 | `handled = true`，保持当前 UI                               |
| handler 抛出异常          | 记录 warning，并以 `handler-error` 消费，避免退出到遮罩下方 |
| handler 返回长期 Promise  | Android 400ms 后 fallback；实现审查中应视为缺陷             |

Handler 不得等待：

- CSS transition 完成；
- API 请求完成；
- 文件删除完成；
- 用户在第二个确认框中作出选择。

如果需要用户确认，handler 应同步打开确认 Dialog并立即报告 handled。

### 9.5 安装时机

建议启动顺序：

```text
initializeShellRuntime()
  -> 创建并挂载 Vue app
  -> installNativeBackNavigation(router)
  -> 安装 nativeBackRequested 订阅
  -> reportFrontendReady()
```

必须保证 `reportFrontendReady()` 晚于订阅安装。这样 Android 把现有 ready 信号同时作为“页面可接收原生返回事件”的生命周期门槛。

HMR dispose、应用卸载或 runtime 重建时必须取消订阅并清空 registry。

## 10. Vue Router 返回策略

### 10.1 最后一级 handler

当没有更高优先级 UI 状态处理本次返回时：

```ts
if (request.canGoBack) {
  router.back();
  return { handled: true, reason: "route-history" };
}

return { handled: false };
```

必须使用 Vue Router 的 `router.back()` / `router.go(-1)`，不能直接调用 `window.history.back()`，以便现有 `onBeforeRouteLeave` 和全局守卫继续生效。

### 10.2 为什么不等待 `afterEach`

Vue Router 的 history traversal 方法不提供可直接等待的导航 Promise。更重要的是，WineStock 已有异步离开守卫会打开 Dialog 并等待用户选择：

- `InboundDraftPage.vue`；
- `ItemsPage.vue`；
- `SubstitutesPage.vue`；
- `ItemCreateDialog.vue`。

如果原生握手等待守卫最终完成，必然超过 400ms，Android 会误判超时并再次执行 `WebView.goBack()`。

因此路由 handler 的“已处理”含义是：**前端已接受本次返回意图并交给 Vue Router**，不是“路由已经完成切换”。

`router.afterEach()` 可以用于记录成功、失败或重定向结果，但不能决定本次 native 应答时机。

### 10.3 离开守卫体验

无浮层、存在未保存草稿时：

```text
第一次返回
  -> router.back()
  -> onBeforeRouteLeave 打开“放弃修改？” Dialog
  -> 前端立即向 Android 报告 handled = true

第二次返回
  -> 通用 ModalDialog handler 关闭确认层
  -> 留在当前页面继续编辑
```

用户点击 Dialog 中“放弃并离开”时，原来的 route guard Promise 正常完成，和普通浏览器行为一致。

### 10.4 OutboundDraftPage 必须整改

实施前 `OutboundDraftPage.vue` 使用同步 `window.confirm()` 处理路由离开。该浏览器原生阻塞对话框无法稳定加入统一 registry，也不符合其它草稿页面已经采用的前端 Dialog 模式。

本任务实施时应将其改成与 `InboundDraftPage` 相同的异步 `ModalDialog + pendingLeaveResolution` 模式。该整改是原生返回路由链路完整验收的前置项。

`ItemAttributeEditor.vue` 中的类型切换 `window.confirm()`不直接由系统返回触发，可列为同批相邻清理，但不应阻塞 native-back 第一版交付。

## 11. 现有前端接入清单

### 11.1 可通过通用组件一次覆盖

| 文件                                            | 状态                      | 接入方案                                                         |
| ----------------------------------------------- | ------------------------- | ---------------------------------------------------------------- |
| `components/ModalDialog.vue`                    | 所有普通、嵌套业务 Dialog | 打开时注册；仅 topmost 可处理；busy 时消费；非 busy 发出 `close` |
| `components/forms/DateTimeField.vue`            | 嵌套日期时间 Dialog       | 由 `ModalDialog` 自动覆盖，无需重复注册                          |
| 所有 `*Dialog.vue`                              | 业务 Dialog               | 只要复用 `ModalDialog` 即自动获得返回行为                        |
| `components/PreviewImage.vue`                   | 全屏图片预览              | `viewerOpen` 时高优先级注册；关闭并恢复触发控件焦点              |
| `components/forms/SelectControl.vue`            | Teleport listbox          | `open` 时注册；返回等价 Escape，关闭并恢复触发控件焦点           |
| `components/attributes/AttributeImageField.vue` | 图片来源/颜色 Popover     | 颜色子层先关闭；再次返回关闭图片来源 Popover                     |

### 11.2 需要在所有者处显式接入

| 所有者文件                         | 临时状态                  | 返回行为                                     |
| ---------------------------------- | ------------------------- | -------------------------------------------- |
| `layouts/AppShell.vue`             | `navOpen`                 | 关闭移动导航 Drawer                          |
| `composables/useAccountPopover.ts` | `accountMenuOpen`         | 关闭账户 Popover                             |
| `pages/LocationsPage.vue`          | `groupPanelOpen`          | 关闭移动库位分组 Drawer                      |
| `pages/InboundDraftPage.vue`       | `selectedLineId`          | 关闭入库明细 Drawer并恢复触发控件焦点        |
| `pages/InboundDraftPage.vue`       | `currentStep === "draft"` | 回到选择物品步骤，不直接离开路由             |
| `pages/OutboundDraftPage.vue`      | `step === "draft"`        | 回到选择物品步骤                             |
| `pages/OutboundDraftPage.vue`      | 未保存草稿路由离开        | 用异步 `ModalDialog` 替换 `window.confirm()` |

### 11.3 现有离开保护的验证项

以下实现主要依赖通用 Dialog 和 router handler，不一定需要新增业务代码，但必须纳入回归：

- `ItemsPage.vue` 的未保存物品编辑；
- `SubstitutesPage.vue` 的未保存替代关系；
- `ItemCreateDialog.vue` 的未保存新建物品；
- `InboundDraftPage.vue` 的自动保存草稿离开确认；
- 所有 nested Dialog 的关闭顺序；
- Dialog 内打开 Select、日期时间选择或图片预览后的逐层返回。

### 11.4 不拦截返回的状态

- `NoticeViewport`：通知不是导航层，不因返回关闭；
- `ServiceUnavailableScreen`：服务状态覆盖层不是可取消浮层；
- 全局 bootstrap/loading 背景：页面未 ready 时由 Android 直接 fallback；
- 普通展开说明、备注字段：除非产品明确把它定义为页面导航层，否则不注册。

## 12. Android 文件级变更清单

| 文件                                                                               | 计划变更                                                                                    |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `android/app/src/main/java/winestock/xiaowine/cc/MainActivity.kt`                  | 返回回调改为协商入口；集中 native fallback；页面/Activity 生命周期清理；继续使用 dispatcher |
| `android/app/src/main/java/winestock/xiaowine/cc/AppConfig.kt`                     | 增加 `NATIVE_BACK_RESPONSE_TIMEOUT_MS = 400L` 等平台常量                                    |
| `android/app/src/main/java/winestock/xiaowine/cc/shell/ShellBridgeHost.kt`         | 发布 `nativeBackRequested`；分发 `resolveNativeBack`；管理页面代次、ready 和 proxy 生命周期 |
| `android/app/src/main/java/winestock/xiaowine/cc/shell/NativeBackRequestBroker.kt` | 新增单 pending 状态机、requestId、timeout、结算和取消                                       |
| `android/app/src/main/java/winestock/xiaowine/cc/shell/RuntimeSnapshotFactory.kt`  | 从真实安装能力生成 `capabilities.nativeBack`                                                |
| `android/app/src/main/assets/shell/bridge.js`                                      | 增加事件 listener set、订阅方法、应答调用和 unavailable bridge 行为                         |
| `android/app/src/test/.../NativeBackRequestBrokerTest.kt`                          | 覆盖 handled、unhandled、timeout、重复、迟到、取消和页面代次                                |
| `android/docs/README.md`                                                           | 记录原生返回协商所有权、超时和 fallback                                                     |
| `docs/code-map/android.md`                                                         | 增加 broker、桥事件和 MainActivity 新职责                                                   |

`ShellBridgeHost.kt` 当前已经较长。native back 状态不应继续直接堆在 method dispatcher 内，broker 拆分是本次可维护性的必要部分。

## 13. Frontend 文件级变更清单

| 文件                                                         | 计划变更                                                                |
| ------------------------------------------------------------ | ----------------------------------------------------------------------- |
| `frontend/src/shell/contract.ts`                             | 增加 request/resolution/ack 类型、capability-gated extension 和结构校验 |
| `frontend/src/shell/runtime.ts`                              | 提供订阅/应答薄包装；按 capability 校验方法；HMR 时取消订阅             |
| `frontend/src/shell/web.ts`                                  | 保持 `nativeBack = false`；提供兼容 no-op 或不暴露 optional extension   |
| `frontend/src/navigation/nativeBack.ts`                      | 新增 registry、优先级、LIFO 调度、router fallback 和单次结算            |
| `frontend/src/composables/useNativeBackHandler.ts`           | 新增响应式注册/注销辅助                                                 |
| `frontend/src/main.ts`                                       | Vue 挂载后、`frontendReady` 前安装 native back 订阅                     |
| `frontend/src/components/ModalDialog.vue`                    | 接入 topmost/busy 返回语义                                              |
| `frontend/src/components/PreviewImage.vue`                   | 接入全屏预览关闭                                                        |
| `frontend/src/components/forms/SelectControl.vue`            | 接入 listbox 关闭和焦点恢复                                             |
| `frontend/src/components/attributes/AttributeImageField.vue` | 接入颜色子层/图片来源 Popover 分层关闭                                  |
| `frontend/src/layouts/AppShell.vue`                          | 接入移动导航 Drawer                                                     |
| `frontend/src/composables/useAccountPopover.ts`              | 接入账户 Popover                                                        |
| `frontend/src/pages/LocationsPage.vue`                       | 接入移动分组 Drawer                                                     |
| `frontend/src/pages/InboundDraftPage.vue`                    | 接入明细 Drawer和页面内步骤                                             |
| `frontend/src/pages/OutboundDraftPage.vue`                   | 接入页面内步骤；用前端异步 Dialog 替换路由离开的 `window.confirm()`     |
| `frontend/docs/mobile-interactions.md`                       | 增加 Android 返回优先级和新临时浮层必须注册的规则                       |
| `frontend/docs/routes.md`                                    | 记录 native back 与 hash history、离开守卫的关系                        |
| `docs/code-map/frontend.md`                                  | 增加 navigation registry 和关键接入点                                   |

## 14. 分阶段实施

### 阶段 0：协议与基线，0.5 人日

- 记录当前设备上 Dialog、Drawer、路由、首页退出的返回行为。
- 固化 request/resolution/ack DTO 和错误规则。
- 建立 Android broker 的 JVM 测试骨架。
- 确认 capability 在完整实现前继续为 false。

### 阶段 1：Android 协商基础设施，1～1.5 人日

- 新增 `NativeBackRequestBroker`。
- 扩展 `ShellBridgeHost` 事件与应答分发。
- 接入页面 generation、frontend ready 和 lifecycle cancel。
- 改造 `MainActivity` 返回入口与 fallback。
- 先通过模拟应答测试 handled、false、timeout 和 duplicate。

### 阶段 2：前端契约与 registry，1～1.5 人日

- 扩展 `contract.ts`、Android shim 和 runtime wrapper。
- 新增 handler registry、优先级和 composable。
- 在 `main.ts` 中建立正确启动顺序。
- 接入 `ModalDialog`、Preview、Select 和图片来源 Popover。

### 阶段 3：应用壳与页面状态，1～1.5 人日

- 接入 AppShell、账户、库位、入库明细 Drawer。
- 接入入库/出库页面内步骤。
- 替换出库草稿离开的 `window.confirm()`。
- 回归 Items、Substitutes、ItemCreate 和 nested Dialog。

### 阶段 4：能力开启、设备验收和文档，0.5～1 人日

- 只有两端全部完成后才把 Android `nativeBack` 改为 true。
- 执行手势导航、三键导航、按键连击、页面刷新和主线程阻塞验证。
- 更新 shell bridge、平台、移动交互和代码地图。
- 清理旧的“原生返回协商尚未实现”注释。

总计预计：4～5.5 人日。主要不确定性来自真实设备 predictive-back 表现、页面内嵌套浮层盘点和现有路由离开守卫回归。

## 15. 测试与验收矩阵

### 15.1 Android JVM 单元测试

至少覆盖：

1. Idle 发起请求并生成唯一 requestId；
2. `handled = true` 只结算一次且不 fallback；
3. `handled = false` 只 fallback 一次；
4. 400ms timeout fallback；
5. Awaiting 中重复返回不产生第二个请求；
6. duplicate resolution 返回 `accepted = false`；
7. late resolution 不影响后续请求；
8. page generation 变化使旧请求失效；
9. pause/destroy 取消 timeout 且不 fallback；
10. 无 proxy / 未 ready 时请求发送失败并立即由 Activity fallback。

### 15.2 前端 registry 测试

当前 frontend 没有测试 runner。推荐为纯 registry 模块引入 Vitest，但该依赖应按项目 checklist 单独审核并只作为 devDependency。

若本次不引入测试依赖，至少通过可注入的纯函数设计和浏览器/设备 smoke 覆盖以下行为：

- priority 降序；
- 同 priority LIFO；
- inactive handler 跳过；
- handler 返回 false 后继续；
- busy handler 消费；
- handler exception 安全消费；
- 同一 request 只 resolve 一次；
- capability false 时不安装订阅；
- route fallback 只在 `canGoBack` 为 true 时调用；
- dispose 后不再处理事件。

### 15.3 真实交互验收

| 场景                     | 第一次返回结果                                   | 第二次返回结果            |
| ------------------------ | ------------------------------------------------ | ------------------------- |
| 首页，无 history、无浮层 | Activity 执行系统返回                            | 不适用                    |
| 普通二级路由，无浮层     | Vue Router 返回上一条 history                    | 继续按上一状态处理        |
| 普通 Dialog              | 关闭 Dialog                                      | 路由返回或退出            |
| busy 提交 Dialog         | 保持 Dialog，不离开页面                          | 提交仍未结束时继续消费    |
| Dialog 内日期时间选择    | 关闭日期时间 Dialog                              | 关闭父 Dialog             |
| Dialog 内 Select listbox | 关闭 listbox                                     | 关闭 Dialog               |
| Dialog 内图片全屏预览    | 关闭图片预览                                     | 关闭 Dialog               |
| 移动 App 导航 Drawer     | 关闭 Drawer                                      | 路由返回或退出            |
| 账户 Popover             | 关闭 Popover                                     | 路由返回或退出            |
| 库位分组 Drawer          | 关闭分组 Drawer                                  | 路由返回或退出            |
| 入库明细 Drawer          | 关闭明细 Drawer                                  | 从填写步骤回到选品步骤    |
| 入库/出库填写步骤        | 回到选品步骤                                     | 触发路由返回/草稿离开确认 |
| Items 未保存编辑         | 编辑 Dialog 请求关闭并出现放弃确认               | 关闭放弃确认，继续编辑    |
| 页面无浮层但有未保存草稿 | Vue Router 守卫打开离开确认                      | 关闭离开确认，保留草稿    |
| JS 主线程无响应          | 约 400ms 后 WebView/Activity fallback            | 不应永久失效              |
| 请求后立即刷新页面       | 旧请求取消，新页面 ready 前走 immediate fallback | 新页面 ready 后恢复协商   |
| 400ms 内连续按两次       | 只执行一个返回语义                               | 第二次在 pending 期被消费 |

### 15.4 平台矩阵

| 维度         | 最低覆盖                                                |
| ------------ | ------------------------------------------------------- |
| Android 版本 | API 26、29/30、33/34、35、36                            |
| 返回方式     | 三键导航返回键、手势导航返回提交、ADB `KEYCODE_BACK`    |
| 方向         | 竖屏、横屏                                              |
| 生命周期     | 冷启动、热恢复、页面刷新、旋转、后台恢复                |
| 页面状态     | 首页、普通路由、Dialog、nested Dialog、Drawer、草稿守卫 |
| 异常         | bridge 不支持、proxy 缺失、JS 卡顿、迟到应答            |

### 15.5 建议验证命令

实现后至少执行：

```powershell
Set-Location frontend
pnpm format:check
pnpm build

Set-Location ..\android
.\gradlew.bat :app:testDebugUnitTest :app:assembleDebug :app:lintDebug --no-daemon
```

构建通过不能替代真实手机返回键与手势 smoke。

## 16. Predictive back 边界

项目当前使用 AndroidX Activity `1.13.0` 和生命周期感知的 `OnBackPressedDispatcher`。第一版应继续走该路径，以获得 AndroidX 对新旧 Android 返回分发的兼容。

第一版只在返回手势**提交**时向前端发送请求：

- 不发送 gesture started/progressed/cancelled；
- 不让 Web UI 跟随手指平移或缩放；
- 不伪造目的页面预览；
- 不直接绕过 AndroidX 注册平台 callback。

验收重点是提交后的结果正确、没有双重返回和卡死。完整 predictive-back 动画需要额外定义连续进度事件、取消回滚、Web 动画性能和路由预览语义，应作为独立后续任务。

## 17. 安全、可靠性与可观测性

### 17.1 安全

- 继续只允许 `https://winestock.internal` 主框架调用桥；
- 非受信任 origin 不能订阅或应答；
- requestId 由原生生成；
- payload 做类型、长度和枚举校验；
- reason 只用于日志，不驱动原生业务分支；
- 不使用 `addJavascriptInterface` 或任意反射方法；
- 不向事件加入 token、路径、用户信息或业务数据。

### 17.2 可靠性

- 单 pending request；
- 单次结算；
- native 硬超时；
- 页面代次隔离；
- lifecycle cancel；
- fallback 时重新读取 WebView history；
- capability 未完成前保持 false。

### 17.3 日志

Debug 日志可记录：

```text
requestId / pageGeneration / dispatched / handled / reason / latencyMs / timeout / accepted
```

Release 日志只保留异常和超时摘要，不记录当前路由名称、表单内容或用户数据。

建议验收指标：

- 正常 handled latency P95 < 100ms；
- 不存在同一 request 两次 fallback；
- 无 unknown request 导致崩溃；
- timeout 后下一次返回仍可正常处理。

## 18. 主要风险与应对

| 风险                                 | 影响                            | 应对                                                         |
| ------------------------------------ | ------------------------------- | ------------------------------------------------------------ |
| capability 提前开启                  | 前端无订阅时每次返回等待超时    | 最后阶段才设 true；按 capability 校验扩展方法                |
| 等待路由守卫完成                     | 草稿确认超过超时并触发双重 back | `router.back()` 后立即 handled；`afterEach` 只做观察         |
| busy Dialog 返回 false               | 提交中穿透到路由或 Activity     | busy 必须消费                                                |
| 同一事件发送两次 resolution          | 重复 fallback 或状态错乱        | 前端单次结算 + Android requestId 去重                        |
| 页面刷新保留旧 replyProxy            | 旧页面应答误结算新请求          | page generation、清 proxy、旧 requestId 返回 accepted=false  |
| 第二次返回排队                       | 一次按键抖动关闭两层或直接退出  | 第一版不排队，Awaiting 时消费                                |
| JS 主线程卡顿                        | 不能及时关闭浮层                | 400ms native fallback；handler 禁止网络和动画等待            |
| 组件漏注册                           | 特定浮层仍被路由返回穿透        | 通用 Modal/Preview/Select 集中接入；按本报告清单做全量 smoke |
| `window.confirm()` 阻塞              | 无法统一处理和自动化验证        | 出库草稿离开确认改为异步 `ModalDialog`                       |
| predictive-back 预览与最终 UI 不一致 | 手势动画体验有限                | 第一版只承诺 commit 正确；完整动画另立方案                   |
| 过度把普通展开状态纳入返回           | 用户返回层级变长                | 仅注册明确的临时导航层，备注/说明默认不拦截                  |

## 19. 回滚策略

不增加长期 feature flag，`capabilities.nativeBack` 本身就是平台能力开关。

如果设备验收出现阻断问题：

1. Android 将 `nativeBack` 恢复为 false；
2. `MainActivity` 在 capability 关闭时立即沿用 `WebView.canGoBack()` / Activity fallback；
3. 前端 registry 可保留但不会收到原生事件，不影响浏览器行为；
4. 不保留两套同时启用的原生返回协议；
5. 修复后重新以完整 Android + packaged frontend 组合开启能力。

回滚不得只移除前端订阅却保留 Android capability 为 true，否则会让每次返回增加 400ms 无效等待。

## 20. 文档同步要求

实现时同步更新：

- `docs/shell-bridge.md`：补充 capability-gated 接口、事件和应答；
- `docs/platforms.md`：Android shell 当前返回能力状态；
- `docs/code-map/android.md`：broker、bridge host 和 Activity 职责；
- `docs/code-map/frontend.md`：native back registry 和组件接入；
- `android/docs/README.md`：超时、fallback、页面代次与生命周期；
- `frontend/docs/mobile-interactions.md`：新浮层注册规则和优先级；
- `frontend/docs/routes.md`：hash history、router guard 和 native back 关系。

实现完成后清理以下过时内容：

- `RuntimeSnapshotFactory.kt` 中“原生返回协商尚未实现”的注释；
- Android 代码地图中“native back 尚未实现”的状态；
- 前端 routes 文档中“Android 返回键行为尚未确认”的条目；
- 与旧 `WebView.goBack()` 单级行为绑定的注释和测试。

## 21. 完成定义

只有同时满足以下条件才算完成：

1. Android 不再把每次返回直接交给 WebView history；
2. capability 为 true 时，前端能收到带 requestId 的原生返回请求；
3. 通用 Dialog、图片预览、Select、主要 Drawer 和页面内步骤全部按优先级工作；
4. 路由离开守卫不会触发 timeout 后的第二次返回；
5. false、timeout、页面未 ready 和 bridge 不支持均能安全 fallback；
6. 连续返回、迟到应答、刷新和 Activity 销毁不产生重复结算；
7. 手势导航与三键导航在真实设备上通过 smoke；
8. frontend build、Android unit/lint/assemble 通过；
9. shell bridge、平台、移动交互和代码地图已同步；
10. 工作区其它用户改动未被回退或混入本任务提交。

## 22. 推荐实施顺序

```text
1. 固化协议 DTO、超时和 broker 单元测试
2. Android request broker + ShellBridgeHost 事件/应答
3. MainActivity 协商入口、fallback 和 lifecycle cancel
4. Android bridge.js + frontend contract/runtime
5. 前端 registry + main.ts ready 顺序
6. ModalDialog / PreviewImage / Select / AttributeImageField
7. AppShell / Account / Locations / Inbound Drawer
8. 入库、出库页面内步骤与出库离开确认整改
9. capability 改为 true
10. 构建、真实设备矩阵、文档和代码地图收尾
```

Android 基础设施与前端消费应在同一功能分支中完成。提交可以按“协议与 broker”“前端 registry 与通用组件”“页面接入”“验证与文档”拆分，但任何可发布构建都不能出现 `nativeBack = true` 而前端尚未完整订阅的中间状态。

## 23. 参考资料

- Android Developers：[OnBackPressedDispatcher](https://developer.android.com/reference/androidx/activity/OnBackPressedDispatcher)
- Android Developers：[OnBackPressedCallback](https://developer.android.com/reference/androidx/activity/OnBackPressedCallback)
- Android Developers：[Add support for the predictive back gesture](https://developer.android.com/guide/navigation/custom-back/predictive-back-gesture)
- Android Developers：[WebViewCompat.addWebMessageListener](<https://developer.android.com/reference/androidx/webkit/WebViewCompat#addWebMessageListener(android.webkit.WebView,java.lang.String,java.util.Set,androidx.webkit.WebViewCompat.WebMessageListener)>)
- Android Developers：[JavaScriptReplyProxy](https://developer.android.com/reference/androidx/webkit/JavaScriptReplyProxy)
- Vue Router：[Programmatic Navigation - Traverse history](https://router.vuejs.org/guide/essentials/navigation.html#traverse-history)
- Vue Router：[Navigation Failures](https://router.vuejs.org/guide/advanced/navigation-failures.html)
- Vue Router：[Global after hooks](https://router.vuejs.org/guide/advanced/navigation-guards.html#global-after-hooks)
- 项目 Shell Bridge：`docs/shell-bridge.md`
- Android 平台职责：`docs/platforms.md`
- Android 代码地图：`docs/code-map/android.md`
- 前端路由：`frontend/docs/routes.md`
- 前端移动交互：`frontend/docs/mobile-interactions.md`
