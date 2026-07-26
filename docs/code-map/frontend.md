# Frontend 代码地图

`frontend` 是 Vue/Vite 共享前端源码和 pnpm 工程，桌面与移动端共享应用框架与业务状态。
它通过 HTTP 调用 core，不拥有平台 WebView 生命周期，Axum 也不能服务其构建产物。
布局改动必须同时检查桌面、断点附近和移动视口。
逐文件职责以各源码文件的中文文件头注释为准；本地图只记录模块所有权、边界和依赖方向。

## 工程入口与启动

- `frontend/package.json`、`vite.config.ts`：pnpm 脚本、Web/Android 双构建模式和 Node test runner 纯逻辑测试入口；Android mode 隔离 `.env*`，接收 Gradle 提供的绝对输出目录并生成可校验 manifest，不固定 Node/pnpm 版本；package.json `appStage` 字段经 define 注入为品牌阶段徽标常量（Web/Android 双模式一致，空串时应用壳徽标隐藏）。
- `frontend/src/main.ts` 与 `src/bootstrap/`：先初始化 Shell 运行快照和动态 API 地址，再按需启动健康检查、会话恢复、跨标签页同步、浮层滚动条和移动视口纠正，安装路由守卫后挂载 Vue，最后才报告 `frontendReady`。
- `frontend/src/App.vue`：根 `RouterView`、服务断连全屏覆盖层、路由切换顶部进度条和全局 Notice 挂载点；服务无关的运行设置路由不受启动门或覆盖层阻塞。
- `frontend/src/env.d.ts`：Vite 环境变量、兼容运行时注入对象和平台 Shell Bridge 注入类型。

## 路由与导航

- `frontend/src/router/`：hash history 路由、鉴权/权限/强制改密守卫和页面元数据；`appRouteCatalog.ts` 是应用壳一级页面名称、权限、导航分组和顺序的唯一声明来源，导航入口按会话权限快照过滤。页面组件懒加载入口集中在 `appPageLoaders.ts` 并在进入应用壳后按权限空闲预取；`navigationPending.ts` 拥有路由切换的稳定等待状态（驱动全局进度条与侧栏乐观高亮）和懒加载 chunk 失败的 Notice 重试。
- `frontend/src/navigation/`：平台无关的原生返回 registry 与请求调度纯逻辑（priority 降序、同级 LIFO、异常安全、requestId 去重），以及接到 Shell Bridge 与 Vue Router 的装配层。

## Shell 运行时与服务可用性

- `frontend/src/shell/`：Shell Bridge v1 契约与运行时结构校验（不能只信任 TypeScript 静态类型）、平台桥/Web fallback 选择（不判断 User-Agent）、版本化 localStorage Web 配置、响应式运行快照编排和局域网地址派生；收到不兼容快照时保留设置页并进入可修复失败态。
- `frontend/src/service/availability.ts`：独立于登录状态的服务健康探测、断连/恢复节奏和窗口焦点补检；API 地址变化时取消旧探测。
- `frontend/src/api/runtime-config.ts`：动态 API 根地址与登录客户端元数据；禁止把全接口监听地址作为访问地址。

## API 层

- `frontend/src/api/client.ts`、`errors.ts`、`pagination.ts`：基于 fetch 的统一 JSON 请求、Bearer 注入、`invalid_access_token` 单次强制刷新重试、XHR multipart 上传进度、API 地址切换时统一中止旧请求，以及后端错误契约解析和泛型分页；只允许相对 API 路径。
- `frontend/src/api/generated/`：由 `pnpm gen:api-types` 从 core 导出的 OpenAPI 生成并入库的 TypeScript 契约类型；只允许生成器写入，已在 `.prettierignore` 排除。`api/contract.ts` 拥有生成契约的桥接辅助（schema 索引与响应 Option 字段必填化），不定义业务 DTO。
- `frontend/src/api/` 其余模块：按业务域拆分的 HTTP 契约与请求函数（auth、users、events、items、substitutes、分类/属性模板、inbound/outbound 及其单据查询、stockApprovals、locations、files、health、dashboard）；全部 HTTP DTO 已通过 `contract.ts` 别名映射到生成类型（导出名不变），查询参数模型与前端本地类型仍手写。立创候选资料、可选参考单价与受控图片读取属于 items 契约，浏览器不直接访问立创域名；图片 Blob 转换为现有待上传图片草稿，保存时继续走 files 上传契约。

## 鉴权会话

- `frontend/src/auth/`：内存 access token 与五态会话模型、跨标签页 Web Locks 协调、到期前自动续期、版本化 refresh token 持久化（绑定 API 根地址）和前端权限快照；任何登出结果都清除本地会话，权限快照只收敛导航与操作入口，不替代服务端实时授权。
- 本机静默会话：Shell 快照携带 `localAuthExchangeToken`（仅 ownership=local）时进入静默免登录模式——refresh 无法恢复会话则用换取凭据调 `/api/auth/local-session` 静默建会话；服务端返回 `local_session_unavailable`（存量库未转换/浏览器场景）按普通匿名回落登录流程，其余失败落 `unavailable` 并入服务可用性覆盖层（不回落登录页），core 重启带来新凭据或健康恢复时自动重试。静默模式下 AppShell 隐藏账户身份与退出登录、隐藏用户管理导航，只保留中性"本机"选项入口；设计见 `docs/implementation-notes/self-hosted-silent-auth.md`。

## 扫码与立创料袋

- `frontend/src/barcode/decoder.ts`：zxing-wasm reader 懒加载、wasm 自托管定位与 QRCode 解码入口；相机帧走速度优先、静态图片走精度优先，模块与 wasm 均不进主包。
- `frontend/src/lcsc/bagCode.ts`：立创料袋二维码文本的解析纯逻辑（node 单测覆盖）；非料袋格式一律返回 null 由调用方静默忽略。
- `frontend/src/lcsc/orderExport.ts` 与 `orderExportFile.ts`：立创商城订单导出表格的解析纯逻辑（行数组进、四字段明细出，node 单测覆盖）与文件读取入口；SheetJS（CDN tarball 版 `xlsx`）只经动态 import 进入订单导入路径，不进主包。
- `frontend/src/components/barcode/`：业务无关扫码层，只回传原文，业务语义由调用方决定（方案见 `docs/implementation-notes/lcsc-bag-scanning.md`）。`BarcodeCameraView.vue` 拥有取景区——摄像头会话、逐帧解码、检测框、设备循环切换（记住选择）、torch、点击触发单次对焦（全局 single-shot 后回连续；Chromium 未实现区域对焦 pointsOfInterest）；`BarcodeScanDialog.vue` 拥有 Dialog 编排——画面上方稳定状态行、图标工具栏、拍照/选图/拖放/粘贴降级。

## 全局反馈

- `frontend/src/notices/` 与 `components/NoticeViewport.vue`：全局 Notice 状态、四种类型、限量、倒计时与暂停交互，展示中的同内容重复请求复用首条不再新增；表单提交校验由 Notice 显示首个错误原因，字段附近保留红框且不插入可见错误行改变表单尺寸。
- `frontend/src/components/ServiceUnavailableScreen.vue`：服务不可用时覆盖业务路由的全屏提示，不执行 HTTP 探测。

## 遥测

- `frontend/src/telemetry/`：匿名使用数据的同意偏好持久化（`consent.ts`，版本化、默认关、不进 Shell 运行配置）与 Microsoft Clarity 按需加载（`clarity.ts`，仅 consent=true 时动态 import SDK 并初始化，项目 ID 随 tag 脚本公开；拒绝或未作答时零请求）。启动入口：应用装配按持久化偏好补启动，初始化向导勾选同意后立即启动。
- 采集事件：业务代码只经 `clarity.ts` 的 `trackTelemetryEvent`（流程完成：出入库提交、物品创建、订单导入、扫码命中）与 `trackTelemetryIssue`（问题 + 会话升级：断连、提交失败、立创查询/订单解析失败、摄像头故障）上报，未同意时全部空操作；事件只有固定名字，不携带业务数据；`identify` 与 deviceName 按隐私承诺不上报。启动时以 Shell 客户端元数据打 platform/appVersion 会话标签。立创查询与物品创建走 `api/items.ts` 唯一入口覆盖全部调用方，用户取消与输错编号不计入。

## 布局与通用组件

- `frontend/src/layouts/AppShell.*`：已登录应用区域唯一的稳定响应式应用框架；同一顶部栏、导航面板和路由出口通过 CSS 在桌面与移动端重排。
- `frontend/src/components/` 顶层：账户弹层与用户摘要、分组导航列表（含导航目标乐观等待反馈）、路由出口切换动画、路由切换顶部进度条、通用 Modal（统一原生返回、嵌套层级与遮罩规则）、密码输入、自动搜索输入、图片预览/应用内查看等跨页面复用件。
- `frontend/src/components/forms/`：FormField 族、Teleport listbox 的 `SelectControl` 和不依赖浏览器原生弹层的日期时间字段；不包含业务校验规则。
- `frontend/src/components/attributes/`：物品侧的单张图片属性控件、无依赖 HSV 颜色选择器、鉴权图片加载和图片草稿/纯色生成/提交阶段批量上传模型。
- `frontend/src/composables/`：表单校验定位、原生返回注册、稳定 pending 指示、目录搜索分页、草稿持久化等页面无关组合逻辑。
- `frontend/src/clipboard/` 与 `directives/copyable.ts`：复制到剪贴板的唯一实现（Clipboard API + execCommand 降级、统一 Notice 反馈规则）与 `v-copyable` 指令（main.ts 全局注册）；组件内禁止手写剪贴板代码，规范见 `frontend/docs/ui-consistency-checklist.md`。

## 业务页面域

每个域由 `pages/` 页面（含同名 `.scss` 和 `pages/<域>/` 模型模块）与 `components/<域>/` 局部组件构成；页面编排请求与状态，局部组件不直接请求 API。

- 认证：`AuthEntryPage`/`LoginPage`/`RegisterPage`/`ChangePasswordPage`；首用户 bootstrap 状态按 API 根地址缓存，强制改密与安全回跳由守卫协同。
- 看板：`DashboardPage` 与原生 SVG 出入库趋势图组件。
- 物品：`ItemsPage` 服务端筛选目录与多页详情 Dialog、立创查询 Dialog（确认后覆盖回填草稿）、目录筛选与列表展示字段维护；读写入口按 `stock.item.read`/`stock.item.manage` 区分。
- 出入库草稿：`/inbound` 与 `/outbound` 共用 `StockDraftPage`，按路由 kind 装配 `components/stock-draft/` 泛型工作台壳与出库分配编辑器；`pages/stock-draft/` 拥有壳契约、两域装配与域样式，行模型/持久化沿用 `pages/inbound-draft/`、`pages/outbound-draft/` 与对应 composable，入库行编辑复用 `components/inbound/InboundLineEditor`；入库域另有 `components/stock-draft/LcscOrderImportDialog` 订单导入预览（匹配/新建/来源勾选与未命中行一键批量创建，写入草稿由入库装配完成）；入库行库位按"同编号唯一库存库位 → 全局默认库位 → 待选择"分层预填，`components/inbound/InboundBatchLocationDialog` 批量补齐剩余未选库位的明细（见 `docs/implementation-notes/inbound-location-prefill.md`）。
- 立创批量创建：`components/items/useBatchLcscItemCreation.ts` 串行批量创建会话（查资料→拉图→套批次模板→创建，单项失败不阻塞、`sku_taken` 自动匹配、中止安全）与 `BatchLcscCreateOptionsDialog.vue` 批次选项 Dialog（整批一个模板/分类/单位，预选全站默认模板）；供订单导入与后续备份导入共用，方案见 `docs/implementation-notes/lcsc-batch-item-creation-and-erp-backup-import.md`。
- 入库单据：`InboundOrdersPage` 单据列表，`components/inbound/` 行编辑器、筛选与列表件。
- 出库单据：`OutboundOrdersPage` 单据列表与 `components/outbound/` 筛选件。
- 审批：两个薄审批路由页与 `pages/approvals/catalog.ts` 领域目录，共同挂载 `components/approvals/StockApprovalWorkspace`（请求与队列协调集中于此）。
- 模板：`TemplatesPage` 分类/物品属性模板双资源工作区，`components/templates/` 字段编辑、复制与两类差异化删除确认。
- 库位：`LocationsPage` 与 `components/locations/` 最多十层分组树、库位 CRUD、删除确认与全局默认库位星标（`is_default`，仅用于入库明细预填）。
- 替代料：`SubstitutesPage` 全局分组治理与确定性星链网络视图（力导向布局 composable），`components/substitutes/` 关系组、共享编辑 Dialog 与 SVG 画布。
- 用户：`UsersPage` 与 `components/users/` 管理表单族；编辑当前账号时锁定权限管理关键权限。
- 审计：`EventsPage` 与 `components/events/` 高级筛选、只读详情和历史 JSON 差异兼容。
- 运行设置：`RuntimeSettingsPage` 无 API 和鉴权依赖；self-hosted 端口由 Shell 自动分配，server-mode 编辑固定端口与监听地址。

## 样式与存储

- `frontend/src/styles/`：foundation 层 token、断点/输入方式/焦点 mixin 和统一安全区变量（业务样式不得绕过 `_safe-area.scss` 直接读原始 env），以及 shared 层视觉原语；组件与页面外观归属各自 `.scss`。
- `frontend/src/storage/`：清理旧版入库草稿遗留 IndexedDB 图片的逻辑；入库草稿不再保存图片。

## 测试

- `frontend/tests/*.test.mjs`：Node test runner 复用现有 TypeScript 转译验证纯逻辑（原生返回 core、局域网地址、启动漏斗、立创映射、料袋码与订单导出解析），不新增测试依赖。`tests/fixtures/lcsc-orders/` 存放本机立创订单导出样本（含真实收货人信息，`.gitignore` 排除不入库）；样本存在时订单解析测试追加真实文件断言，否则自动跳过。

## 文档

前端页面、交互、视觉与规范文档统一由 [`frontend/docs/README.md`](../../frontend/docs/README.md) 索引维护，本地图不复制文档清单；UI 改动的阅读顺序也在该索引中。

## 平台边界

- `desktop/` 当前不是正式 Tauri shell，也不属于 Cargo 工作区。
- 正式 Desktop/Android shell 应在前端挂载前注入运行时 API 地址和客户端元数据。
- 不要从当前脚手架推断最终平台包名、WebView 协议或资源目录。
