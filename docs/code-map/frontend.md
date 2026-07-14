# Frontend 代码地图

`frontend` 是 Vue/Vite 共享前端源码和 pnpm 工程。
它通过 HTTP 调用 core，不拥有平台 WebView 生命周期，Axum 也不能服务其构建产物。
未明确要求移动端时，当前界面改动默认只作用于桌面端。

## 工程入口

- `frontend/package.json`：Vue、Vite、Vue Router 和 `sass-embedded` 依赖；安装和脚本统一使用 pnpm。
- `frontend/src/main.ts`：注册 token 与网络失败回调，启动服务监控、跨标签页同步和自动刷新调度，安装鉴权守卫、提前启动统一会话初始化并挂载 Vue Router。
- `frontend/src/App.vue`：前端根 `RouterView`、服务断连全屏阻断层和全局 Notice 挂载点，不拥有具体页面布局或探测调度。
- `frontend/src/env.d.ts`：Vite 环境变量和平台运行时注入对象类型。

## 路由与布局

- `frontend/src/router/index.ts`：hash history、根应用壳嵌套路由、OpenAPI 业务域页面、鉴权页面、用户管理路由，以及未匹配路径返回库存总览的 catch-all 重定向；应用壳页面的 `meta` 从统一路由目录生成。
- `frontend/src/router/appRouteCatalog.ts`：应用壳一级页面名称、权限、导航分组、顺序和平台可见性的唯一声明来源；不创建 Router 或执行权限判断。
- `frontend/src/router/meta.d.ts`：页面标题、`requiresAuth`、`requiredPermission` 和强制改密页面放行元数据。
- `frontend/src/router/guards.ts`：等待会话初始化、拦截匿名和缺少页面权限的访问、强制改密导航、安全解析登录回跳，并监听会话和权限变化。
- `frontend/src/router/navigation.ts`：从统一路由目录生成应用壳一级导航，并按当前会话权限快照和平台可见性过滤入口，不独立维护页面名称或权限。
- `frontend/src/composables/useResponsiveShell.ts`：按 `768px` 断点只挂载当前桌面或移动 Shell，并提供操作时即时核对当前 Shell 的无副作用判断；冷启动和宿主 resize 同时校验布局视口、有效宿主窗口与可视视口宽度，忽略小于 `320px` 的异常 `outerWidth`。
- `frontend/src/composables/useStablePendingIndicator.ts`：把即时异步等待转换为延迟显示和最短展示的稳定视觉状态，不执行具体请求。
- `frontend/src/composables/useInboundItemCatalog.ts`：入库物品目录的搜索取消、服务端滚动分页、ID 去重、到底和重试状态。
- `frontend/src/composables/useInboundDraftPersistence.ts`：版本化入库草稿序列化/恢复和浏览器原生关闭提示；普通字段保存在 `localStorage`，待上传图片通过独立存储模块写入 IndexedDB。
- `frontend/src/storage/inboundDraftImageStore.ts`：入库草稿本地图片 Blob 的 IndexedDB 存取，不保存普通字段或执行上传。
- `frontend/src/layouts/AppShell.vue`：已登录应用区域的响应式 Shell 选择。
- `frontend/src/layouts/DesktopShell.vue`：桌面品牌栏、左侧导航、顶部右侧账户摘要、紧凑账户弹层和嵌套路由出口；不展示未经真实数据支持的服务状态。
- `frontend/src/layouts/MobileShell.vue`：移动顶部栏、顶部右侧用户头像、紧凑账户弹层、带统一 motion token 动画的 Drawer 和嵌套路由出口。
- `frontend/src/components/AccountUserSummary.vue`：桌面顶部与移动账户弹层复用的只读用户头像和名称摘要。
- `frontend/src/components/AccountPopover.vue`：桌面和移动共用的账户操作弹层；移动端可补充用户摘要，组件不直接读取或清理会话。
- `frontend/src/components/AppNavigationList.vue`：桌面侧栏与移动 Drawer 共用的分组导航、线性图标和选中态渲染。
- `frontend/src/components/RouteContentView.vue`：桌面和移动 Shell 共用的嵌套路由出口，以及复用统一 motion token 的页面切换动画。
- `frontend/src/composables/useAccountPopover.ts`：桌面和移动共用的账户弹层状态、路由变化关闭和 Escape 关闭逻辑。
- `frontend/src/composables/useShellLogout.ts`：桌面和移动应用壳共用的退出编排、错误反馈和登录页跳转。
- `frontend/src/components/ModalDialog.vue`：通用模态结构、关闭行为、基础焦点进入与返回，以及复用统一 motion token 的打开和关闭动画；支持业务 Dialog 内二级设置窗口使用独立嵌套层级、较浅遮罩和紧凑尺寸。
- `frontend/src/components/PasswordInput.vue`：登录、注册、改密和用户管理共用的密码输入呈现控件，统一显示/隐藏、焦点恢复和无障碍状态，不校验或持久化密码。
- `frontend/src/components/SearchField.vue`：目录页面共用的自动搜索输入，统一搜索图标、输入草稿、防抖触发和清空恢复；不请求数据或管理分页。
- `frontend/src/components/forms/SelectControl.vue`：项目级底层选择控件，使用 Teleport listbox 统一触发器、展开浮层、键盘操作、焦点、错误和禁用状态，并保留数字、布尔值与空值的绑定类型；不拥有字段标题或业务选项。
- `frontend/src/components/forms/`：通用 `FormField`、`FormInput`、`FormSelect` 和 `FormTextarea` 字段组件；`FormSelect` 组合 `SelectControl`，统一标题、必填标记、提示、错误状态和无障碍关联，不包含业务校验规则。
- `frontend/src/composables/useFormValidation.ts`：为当前表单组件子树注册字段位置、清理单字段错误并自动滚动聚焦首个错误；校验结果仍由页面、会话或业务模型产生。
- `frontend/src/components/inbound/InboundLineEditor.vue`：正式入库工作台的明细抽屉，编辑批次、有效期、入库模板和本次收货属性，并在移动端切换为全屏编辑面板。
- `frontend/src/components/inbound/InboundCatalogStep.vue`：正式入库流程的第一步，完成物品搜索、分页浏览和按物品去重的加入或移出操作。
- `frontend/src/components/inbound/InboundDraftStep.vue`：正式入库流程的第二步，编辑来源、备注、数量、价格和分组库位，并展示逐行完整性状态。
- `frontend/src/components/attributes/AttributeImageField.vue`：物品与入库共用的单张图片属性控件；通过带视口避让和统一动效的锚定浮层选择本地文件或纯色图，已选图片使用通用组件全屏预览，并保留独立的更换与删除入口，同时完成签名预检和本地预览；不在编辑阶段上传。
- `frontend/src/components/attributes/AttributeColorPicker.vue`：图片字段共用的无依赖 HSV/HEX 颜色选择器，提供饱和度与亮度平面、色相滑轨、HEX 输入、快捷色板和 Pointer Events/键盘交互；只输出颜色并通知应用，不生成图片。
- `frontend/src/components/attributes/imageDraft.ts`：统一图片草稿状态、随机色板、Canvas 纯色 PNG 生成和表单提交阶段的批量上传。
- `frontend/src/components/PreviewImage.vue`：普通图片与全屏查看两态通用组件，拥有遮罩、关闭、焦点返回和背景滚动锁定，不请求文件或编辑图片。
- `frontend/src/components/attributes/AuthenticatedImage.vue`：通过鉴权文件接口加载只读物品主图并管理 Blob URL，可按调用方要求组合通用全屏预览。
- `frontend/src/components/items/`：物品基础资料、主图、可选属性模板、任意属性编辑控件和替代关系；已有物品工作区使用资料/库存/替代关系多页 Dialog，库存和替代关系均按需加载，新建会话只挂载资料编辑器。
- `frontend/src/components/items/ItemCatalogFilterDialog.vue`：物品目录分类、属性模板和动态字段多选筛选草稿；负责取消、清除、折叠候选和应用事件，不请求目录或管理分页。
- `frontend/src/components/NoticeViewport.vue`：右上角 Notice 视口、类型状态色竖条、关闭按钮、倒计时条、统一 motion token 动画及悬浮或键盘聚焦暂停交互。
- `frontend/src/components/ServiceUnavailableScreen.vue`：服务不可用时替换路由内容的全屏提示和手动重试入口；不执行 HTTP 探测。
- `frontend/src/components/users/`：创建用户、权限编辑、临时密码、启停和软删除确认表单。
- `frontend/src/components/users/UserPermissionsDialog.vue`：分类权限选择器；编辑当前账号时锁定权限管理和权限定义读取两项关键权限，不调用权限 API。
- `frontend/src/components/users/UserListToolbar.vue`：用户列表搜索、状态选择框筛选、结果数量、刷新图标和创建入口；不请求 API。
- `frontend/src/components/users/UserActionsDialog.vue`：移动端用户管理操作面板，收敛普通操作并单独呈现删除入口；不调用业务 API。

## 全局操作反馈

- `frontend/src/notices/notice.ts`：全局响应式 Notice 状态和调用 API，提供成功、提示、警告、错误四种类型以及可选详情和点击回调，负责限量、自动消失、暂停和恢复倒计时。
- 登录、注册、修改密码、退出和用户管理操作统一调用 Notice；字段级校验仍保留在对应表单附近。
- 页面级加载失败可以保留持续错误状态，同时调用 Notice 告知本次请求失败。

## API client 与鉴权状态

- `frontend/src/api/runtime-config.ts`
  - 解析平台注入的 `window.__WINESTOCK_RUNTIME_CONFIG__` 或 Vite 环境变量。
  - 校验 API 根地址必须为 HTTP/HTTPS，禁止把 `0.0.0.0` 作为访问地址。
  - 提供登录请求所需的客户端类型、设备名称和版本号。

- `frontend/src/api/client.ts`
  - 基于原生 `fetch` 实现统一 JSON 请求、查询参数、Bearer token 注入和 204 响应处理。
  - 只允许相对 API 路径，避免 access token 被发送到外部绝对地址。
  - 收到 `invalid_access_token` 时最多强制 refresh 并重试一次；不直接持久化 token，也不决定页面提示。
  - 真实网络连接失败时通知全局服务监控；调用方主动取消请求不触发断连状态。
  - 支持鉴权 Blob 响应，并通过 XHR multipart 入口报告真实上传进度和复用 token refresh。

- `frontend/src/api/errors.ts`
  - 定义配置、网络、响应解析和非 2xx 错误类型。
  - 解析后端 `{ error: { code, message, details } }` 契约和字段校验详情。

- `frontend/src/api/auth.ts`
  - 定义注册、登录、refresh、logout、当前用户改密、用户摘要和 token 响应 DTO。
  - 当前实现 `POST /api/auth/register`、`POST /api/auth/login`、`POST /api/auth/refresh`、`POST /api/auth/logout` 和 `POST /api/auth/me/password`。

- `frontend/src/api/users.ts`
  - 定义用户分页、用户详情、权限定义和管理请求 DTO。
  - 实现用户查询、后续用户注册、启停、软删除、权限替换、临时密码和权限定义接口。

- `frontend/src/api/items.ts`
  - 分别定义物品创建、更新和软删除命令、库存目录、结构化筛选与筛选值、轻量选择、编辑资料、库存详情和批次分页契约；入库工作台只持有轻量选择响应。

- `frontend/src/api/substitutes.ts`
  - 定义指定物品替代关系查询和整体替换请求；替代关系的循环、重复和自引用校验由 core 服务负责。

- `frontend/src/api/templateFields.ts`
- `frontend/src/api/itemCategories.ts`
- `frontend/src/api/itemAttributeTemplates.ts`
- `frontend/src/api/inboundTemplates.ts`
  - 定义分类、物品属性模板和入库模板的独立查询契约。

- `frontend/src/api/inbound.ts`
  - 定义入库提交模式、创建响应、库位和模板字段 DTO，查询真实库位与模板详情，并提交待审批或直接入库请求。

- `frontend/src/api/files.ts`
  - 定义图片文件 DTO、PNG/JPEG/WebP 文件头预检、15MB 限制和上传/读取/删除接口。

- `frontend/src/api/health.ts`
  - 无鉴权调用 `GET /api/health` 并校验固定健康响应，用于独立于登录状态的服务可用性探测。

- `frontend/src/api/dashboard.ts`
  - 定义库存摘要、呆滞物品和每日趋势 DTO，并实现总览与趋势只读接口。

- `frontend/src/service/availability.ts`
  - 启动后立即探测服务；可用时每 15 秒、不可用时每 5 秒检查一次，并在窗口聚焦、页面恢复可见或网络恢复时补检。
  - 公开服务状态、探测状态和成功序号；不启动 Axum、不管理 token，也不决定页面布局。

- `frontend/src/auth/permissions.ts`
  - 定义用户管理稳定权限代码（包含 `user.delete`）和前端权限快照判断。
  - 只收敛导航和操作入口，不替代 Axum 实时授权。

- `frontend/src/auth/session.ts`
  - 在内存保存 access token、预计过期时间和用户摘要；refresh token 只从统一 localStorage 读取。
  - 公开五态会话初始化模型、单一初始化/refresh/logout Promise，并区分明确匿名和服务暂不可用。
  - 改密成功后以不可变方式清除当前用户摘要中的强制改密标记，保留现有 token 和权限。
  - 当前用户权限被管理接口修改后，同步内存权限快照，使导航和操作立即更新。
  - 在跨标签页锁内读取最新 refresh token，执行轮换或服务端吊销；任何登出结果都会清除本地会话。
  - 监听其它同源标签页移除持久 token，并同步清除当前内存会话和进入匿名状态。

- `frontend/src/auth/auto-refresh.ts`
  - 按 access token 预计到期时间安排一次性定时器，在到期前约 50 至 60 秒主动 refresh。
  - 网络或服务暂不可用时延迟重试，并在窗口焦点、页面重新可见和网络恢复时立即补检。
  - 登出、匿名或尚未建立会话时取消定时任务；不保存 token，也不绕过会话层的 Promise 和跨标签页锁。

- `frontend/src/auth/coordination.ts`
  - 使用同一个 Web Locks API 锁在同源标签页和 Worker 间串行执行 refresh 与 logout。
  - Web Locks 不可用时直接执行任务，由会话层的最新 token 比较和单次重试兜底。

- `frontend/src/auth/storage.ts`
  - 使用版本化 `localStorage` 记录统一持久化 refresh token，并绑定获取 token 的 API 根地址。
  - 不保存 access token、密码或用户资料；损坏和不兼容记录会被清除。
  - 支持按预期 token 条件清除，并提供其它同源标签页移除记录的 `storage` 事件订阅。

## 页面

- `frontend/src/pages/LoginPage.vue`：桌面和移动共用的用户名密码登录页面，调用登录 API、映射字段错误、安全恢复内部目标并显示本机退出警告。
- `frontend/src/pages/RegisterPage.vue`：桌面和移动共用的首个用户注册页面，处理密码确认、错误映射和注册后自动登录流程。
- `frontend/src/pages/ChangePasswordPage.vue`：桌面和移动共用的当前用户改密页面，处理强制改密、主动改密、错误映射、原目标恢复和退出。
- `frontend/src/pages/DashboardPage.vue`：库存摘要、趋势周期、后台刷新、呆滞物品和错误状态编排，只展示服务端真实统计。
- `frontend/src/components/dashboard/DashboardTrendChart.vue`：按容器宽度自适应的原生 SVG 出入库双曲线、坐标轴、桌面悬浮提示和窄屏触控详情，不请求 API。
- `frontend/src/pages/ItemsPage.vue`、`ItemsPage.scss`、`pages/items/model.ts`：承担库存监控和补货判断的物品目录；桌面使用固定身份/库存列与纵向复合单元格，移动使用无横向表格的库存项目。关键词、库存状态、高级结构化筛选、计数、排序和分页来自服务端；所有具备 `stock.item.read` 的用户可从目录行或详情图标进入资料和库存详情，只有 `stock.item.manage` 用户才能看到新建、删除和保存入口，详情 Dialog 才可编辑。
- `frontend/src/components/items/ItemCatalogAttributeDialog.vue`：维护现有物品模板中最多三个列表展示字段，不编辑模板字段结构。
- `frontend/src/pages/InboundDraftPage.vue`、`InboundDraftPage.scss`：`/inbound` 正式多明细入库工作台；编排跨设备双步骤流程、带稳定舞台和方向语义的 `out-in` 步骤动画、草稿恢复、物品去重、权限控制的流程内物品新建、动态模板、图片上传、提交确认和后端错误定位。
- `frontend/src/pages/PlaceholderPage.vue`：入库单、出库、审批、库位、分类与模板、替代关系和审计日志共用的无数据占位页；页面标题读取路由元数据，只展示路由职责与对应 OpenAPI 范围。
- `frontend/src/pages/inbound-draft/model.ts`：入库草稿 `lineId` 模型、模板字段校验、file 引用、提交模式和请求构造规则。
- `frontend/src/pages/inbound-draft/presentation.ts`：入库草稿页错误文案、网络错误映射和数值展示格式化。
- `frontend/src/pages/items/fileCleanup.ts`：物品草稿切换、字段删除和类型变化时清理未绑定图片。
- `frontend/src/pages/UsersPage.vue`：用户列表、搜索、状态筛选、稳定刷新、分页、创建、启停、软删除、权限和临时密码操作编排。

## 样式和文档

- `frontend/src/styles/index.scss`：全局 SCSS 入口，仅装配 foundation、shared 和认证页面族样式。
- `frontend/src/styles/foundation/`：浅色视觉与 motion token、基础渲染和减少动态效果适配。
- `frontend/src/styles/shared/`：品牌、控件、表单、页面骨架和用户对话框共享视觉原语。
- `frontend/src/layouts/*.scss`：AppShell、DesktopShell 和 MobileShell 各自拥有的布局与断点规则。
- `frontend/src/components/**/*.scss`：导航、账户、模态框和用户管理组件各自拥有的外观与响应式规则。
- `frontend/src/pages/*.scss`：具体业务页面拥有的表格、移动列表、状态和分页样式。
- `frontend/docs/page-framework.md`：页面框架和桌面/移动所有权。
- `frontend/docs/routes.md`：路由、history 策略和鉴权守卫状态。
- `frontend/docs/api-client.md`：API 地址、请求行为、错误契约和会话边界。
- `frontend/docs/auth-logout-and-route-guards.md`：登出 API/UI、会话状态、路由守卫、多标签页退出实现和验收记录。
- `frontend/docs/user-management.md`：用户管理页面、权限边界、API、密码安全和验收重点。
- `frontend/docs/visual-style.md`：当前视觉规则。
- `frontend/docs/ui-design-guidelines.md`：后续业务页面统一采用的页面骨架、三段式列表、工具栏、表单、浮层和响应式设计规则。
- `frontend/docs/ui-consistency-checklist.md`：业务页面在状态、视口、真实尺寸、溢出、动效和可访问性上的实施与量化验收清单。
- `frontend/docs/async-state-transitions.md`：加载、恢复、后台刷新和错误切换的防闪烁呈现规则。

## 平台边界

- `desktop/` 当前不是正式 Tauri shell，也不属于 Cargo 工作区。
- 正式 Desktop/Android shell 应在前端挂载前注入运行时 API 地址和客户端元数据。
- 不要从当前脚手架推断最终平台包名、WebView 协议或资源目录。
