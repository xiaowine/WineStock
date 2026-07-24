# 移动端 WebView 与安全区规范

本文件定义共享前端在浏览器、Android WebView 和未来 Desktop shell 中消费视口安全区的统一规则。
它只约束前端布局，不拥有 Android WindowInsets 采集、WebView 生命周期或 Shell Bridge 传输。

## 数据来源

foundation 层的 `styles/foundation/_safe-area.scss` 提供唯一消费入口：

- `--shell-safe-area-inset-top/right/bottom/left` 是平台壳发布的 CSS 值，默认均为 `0px`；
- `--safe-area-top/right/bottom/left` 分别取浏览器 `env(safe-area-inset-*)` 与 shell 值的较大者；
- 原始 `env()` 只能出现在该 foundation 文件中，业务组件不得直接使用或与 shell 值相加。

Android shell 的 `WebViewportInsetsPublisher` 读取 `systemBars | displayCutout`，
按 display density 转成 CSS 像素后写入 shell 变量。IME/键盘不是通用安全区来源，
继续由 `adjustResize`、visual viewport 和页面滚动行为处理。

## 布局规则

1. `html`、`body`、`#app`、遮罩和背景层保持 full bleed，不添加全局安全区 padding。
2. 顶栏、Drawer header、关闭按钮、表单、固定底部操作区和通知等可读/可操作内容，
   按靠近的边消费对应 `--safe-area-*`。
3. 普通滚动宿主要为底部安全区保留可滚动尾部，确保最后一项能滚到导航栏上方。
4. 横屏或 display cutout 必须同时考虑 left/right；不能只处理 top/bottom。
5. Dialog/Sheet 的遮罩可以覆盖全视口，面板和操作区按语义避让；不要通过复制平台判断
   建立第二套布局。
6. 图片查看使用应用内固定层（`InAppImageViewer`），不隐藏系统栏、不调用 Fullscreen；
   遮罩 full-bleed 铺满（含系统栏区域），关闭按钮与图片仍按 `--safe-area-*` 避让；不扩展 Shell Bridge。

## 受影响组件清单

当前统一变量已覆盖：

- 应用壳顶栏、内容滚动区和移动导航 Drawer；
- 登录/注册/修改密码与运行设置；
- 服务不可用全屏状态；
- 通用 Dialog、审批/物品/替代关系固定操作区；
- Notice、图片预览、库位 Drawer、审计日志触底区域和入库工作台操作栏。

新增固定定位或全屏组件时，必须在对应组件样式中说明安全区消费位置，并避免重新引入
`env(safe-area-inset-*)`。

## Android 原生返回优先级

Android 声明 `capabilities.nativeBack = true` 时，前端通过统一 registry 处理一次返回请求；普通浏览器
和 Vite 环境保持 capability=false，不模拟系统返回。`frontendReady()` 必须晚于事件订阅安装。

处理顺序固定为：

1. `500`：Select listbox、图片来源与颜色等临时子浮层；
2. `450`：图片全屏预览；
3. `400`：最上层 Dialog，busy 时消费但不关闭；
4. `300`：移动 Drawer 与 Popover；
5. `200`：入库/出库工作台页面内步骤；
6. `100`：Vue Router history；
7. 无 handler 处理时回复 `handled=false`，由 Android 重新判断 WebView/Activity fallback。

同优先级按最近打开顺序处理。新增可取消的 Teleport 浮层、Drawer、Popover、全屏预览或页面内步骤时，
必须通过 `useNativeBackHandler` 注册；关闭时立即结算，不等待 CSS 动画、网络请求、文件操作或用户在后续
确认框中的选择。需要确认时应同步打开 `ModalDialog` 并把本次返回视为已处理。

`NoticeViewport`、服务不可用覆盖层和 bootstrap/loading 背景不是导航层，不注册原生返回。

## 验收

至少检查以下视口和状态：

- 浏览器桌面 `1440 × 900`、断点附近 `768px`、移动 `390 × 844`；
- Android 手势导航和三键导航、竖屏和横屏、普通屏和挖孔屏；
- 冷启动、页面刷新、Drawer/Dialog/Notice/图片预览、服务不可用、固定底部操作；
- 最后一项内容可滚动到导航栏上方，标题/关闭按钮/输入框不被 cutout 遮挡；
- 计算样式中 shell 与 env 没有相加造成的双重空白，控制台无新增错误。
- 逐层验证 Select/图片子浮层、预览、nested Dialog、导航/库位/明细 Drawer、账户 Popover、
  入库/出库步骤和未保存草稿离开确认；400ms 内连续提交不得一次关闭两层。
