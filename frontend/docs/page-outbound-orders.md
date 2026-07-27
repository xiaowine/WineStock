# 出库单页面实施设计

本文定义 `/outbound/orders`“出库单”查询页面的前端实施边界。它与入库单页面共享列表、筛选、触底追加和只读详情模式，但业务语义相反：创建出库单只形成待审批请求，审批成功才扣减指定批次或 FIFO 库存。新建出库属于 `/outbound`，审批/拒绝属于 `/approvals/outbound`；本页不得直接变更库存。

## 目标与约束

- 让 `stock.outbound.read` 用户定位单据，并明确区分待审批、已出库、已拒绝及其库存影响。
- 前端只能经 HTTP 调用 core；不得逐行请求物品、用户或批次来补显示信息，也不得硬编码服务地址。
- 服务端 `page`、`page_size` 仅是取数协议；页面统一使用尾部哨兵触底追加，禁止上一页/下一页翻页器。
- 复用 `SearchField`、`ModalDialog`、`DateTimeField`、`AuthenticatedImage`、`useStablePendingIndicator` 和 Notice；不引入表格、状态管理或日期第三方依赖。
- 桌面、约 768px 和 390×844 共享同一筛选、总数、加载与权限状态；仅改变布局。

## 现有 HTTP 契约

| 接口                                              | 权限                     | 用途                                                                                    |
| ------------------------------------------------- | ------------------------ | --------------------------------------------------------------------------------------- |
| `GET /api/outbound`                               | `stock.outbound.read`    | 服务端分页列表，已有 `page`、`page_size`、`item_id`、`date_from`、`date_to`、`search`。 |
| `GET /api/outbound/filter-values`                 | `stock.outbound.read`    | 历史筛选候选；首版不以它伪造本地筛选。                                                  |
| `GET /api/outbound/{id}`                          | `stock.outbound.read`    | 打开详情后的完整单据与明细。                                                            |
| `POST /api/outbound`                              | `stock.outbound.create`  | 新建 pending 出库单；本页只跳转，不调用。                                               |
| `POST /api/stock-approvals/outbound/{id}/approve` | `stock.outbound.approve` | 审批并按指定批次或 FIFO 扣库存；仅审批页调用。                                          |
| `POST /api/stock-approvals/outbound/{id}/reject`  | `stock.outbound.approve` | 拒绝 pending 单据；仅审批页调用。                                                       |

`OutboundResponse` 应提供单据 ID、目的地 `destination`、状态、备注、创建/审批/拒绝用户 ID 与时间，以及明细的物品 ID、数量、可选 `batch_id`、批次号、单价/成本或业务金额字段。列表和详情均使用同一响应时，列表只展示轻量摘要，详情再按需读取完整属性。

### 本页实施前需补齐的 core 契约

入库单已证明只返回 `item_id` 无法让用户识别收发物品；出库单页面上线前必须让每条明细投影以下物品身份资料：

- `item_name`、`item_sku`、`item_unit`、`item_image_file_id`；
- 若已有批次，返回 `batch_no`、有效期及该批次当前可读的必要标识；
- `GET /api/outbound` 的查询与 `total` 同时支持稳定 `status=pending|approved|rejected` 参数。非法状态返回现有 `400 invalid_request`。

物品名称、编码、单位和图片应通过 repository 查询批量投影，不能在前端按行请求。若产品要求历史资料不随物品改名变化，再单独设计出库明细展示快照和迁移/回填策略。

## 权限与库存语义

| 能力                          | 页面行为                                                     |
| ----------------------------- | ------------------------------------------------------------ |
| 无 `stock.outbound.read`      | 路由守卫离开页面，禁止请求。                                 |
| `stock.outbound.read`         | 查询、筛选、查看详情和触底追加。                             |
| 另有 `stock.outbound.create`  | 顶部显示“新建出库”，跳转 `/outbound`。                       |
| 另有 `stock.outbound.approve` | pending 详情显示“前往出库审批”，跳转 `/approvals/outbound`。 |

- `pending`：尚未扣减库存；详情必须明确“等待审批，库存未扣减”。
- `approved`：审批时已扣减库存；显示审批时间，不使用“已入库”等入库文案。
- `rejected`：未扣减库存；显示拒绝时间和“已拒绝，库存未扣减”。
- 本页不出现“直接出库”、通过、拒绝、重新扣减或库存调整按钮。

## 信息架构与桌面界面

```text
页面标题“出库单” + 说明
├─ 工具栏：关键词搜索 | 结果数 | 筛选 Dialog | 刷新 | 新建出库
├─ 连续工作区
│  ├─ 三段式出库单列表（桌面）/ 单列项目（移动）
│  └─ 尾部：加载更多、失败重试、已加载全部
└─ 出库单详情 ModalDialog
   ├─ 状态/目的地上下文
   ├─ 单据摘要
   └─ 出库物品与批次明细
```

桌面列表固定为“单据与目的地 | 出库物品 | 状态与操作”三段：

1. 身份段：`出库单 #id`、目的地、创建时间。
2. 物品段：首个物品主图、名称、编码、数量和单位；多明细显示“等 n 项”；第二行显示 `n 条明细` 与出库总量。混合单位不得伪造合计。
3. 判断段：状态标签、审批/拒绝时间、线框圆圈信息详情图标。整行、Enter、Space 与图标均可打开详情，图标阻止冒泡。

目的地为空或历史异常时安全回退为“未记录目的地”；长名称/编码截断但保留 title。金额只有在 API 明确定义出库价格语义时才显示，不能将批次成本误称为销售金额或实时库存价值。

## 工具栏与筛选 Dialog

- `SearchField` 占弹性宽度，搜索单号、目的地、物品名称/编码、批次号和后端明确纳入的可搜索字段；防抖后更新 URL query 并重载第 1 页。
- 筛选图标打开 `OutboundOrderFiltersDialog`，草稿字段为状态、开始时间、结束时间；日期使用项目 `DateTimeField`，本地值转换 ISO UTC 后写入 `status`、`date_from`、`date_to`。
- 图标徽标显示状态和两个时间条件的启用数；“重置”仅重置草稿，“应用筛选”才改变 URL 与请求。
- URL 只保存实际可服务端执行的查询；无效日期、非法状态和开始晚于结束在前端回退/校验，不能保留坏参数。

## 列表异步与触底追加

- 首次、筛选变化：请求第 1 页；超过 200ms 显示稳定加载，显示后至少 350ms。
- 后台刷新：保留旧结果、总数和滚动上下文，仅弱化列表并旋转刷新图标；成功后替换第 1 页。
- 尾部 `IntersectionObserver` 使用列表滚动容器为 root，`rootMargin: 240px 0px`；只在未加载、非 pending 且 `page < total_pages` 时请求下一页。
- 追加按订单 ID 去重；旧 `AbortController` 响应不得覆盖新筛选或新详情。
- 尾部固定显示“正在加载更多”“加载失败，点击重试”“继续向下滚动加载”或“已加载全部 n 条出库单”。
- 零结果移除表头/空列表，保留工具栏和列表尾部；区分“暂无出库单”与“没有符合筛选条件的出库单”。

## 响应式列表

- `min-width: 901px` 保留桌面三段式列表和工作区外框。
- `768px–900px` 在侧栏占宽后直接使用紧凑单列条目，不压缩桌面三段，也不产生横向滚动。
- `390×844` 由 Shell 独占页面标题并解除工作区装饰性外框；搜索、总数、筛选、刷新和有权限时
  的新建图标留在连续工具栏，图标触控区为 `40px`。
- 移动结果背景延伸到页面内侧边缘，卡片保持标准内容缩进；编号、状态、去向、时间、首件物品和
  明细摘要完整可见。
- 整张卡片、Enter 和 Space 打开只读详情；图片预览阻止冒泡，不能同时打开图片与单据 Dialog。

## 详情 Dialog

使用宽 `ModalDialog`，移动端占可用视口。打开后先保留列表行的状态、目的地和编号，再请求 `GET /api/outbound/{id}`；不从列表猜测批次、审批信息或动态字段。

- context：状态标签、目的地、创建时间。
- 摘要：创建时间、状态与库存语义、备注、创建/审批/拒绝用户 ID 与时间。没有安全的姓名投影时只显示“用户 #id”，不发起 N+1 用户请求。
- 出库物品：每条为清晰边框条目，左侧受控主图，主标题物品名称，副标题编码/单位/物品 ID，右侧数量；下方显示库位/批次、有效期、批次指定或 FIFO 语义、单价或成本（仅当契约明确）。
- 批次不存在或 `batch_id` 为空时显示“按 FIFO 分配（审批时确认）”，不能伪造具体批次；pending 不能声称已扣某批库存。
- `403` 关闭详情并保留列表上下文；`404` 提示不可读取并刷新第 1 页；网络错误保留 Dialog 与重试。

## 可复制字段

详情 Dialog 物品行的名称与编号经 `v-copyable` 提供点击复制（无常驻图标，
与入库单详情一致）；规范见 [`ui-consistency-checklist.md`](ui-consistency-checklist.md)。

## 建议模块

```text
frontend/src/api/outboundOrders.ts
  出库单列表/详情 DTO 与 HTTP 请求。
frontend/src/pages/OutboundOrdersPage.vue/.scss
  URL、筛选、触底追加、列表、详情会话。
frontend/src/components/outbound/OutboundOrderFiltersDialog.vue/.scss
  状态与时间草稿；不请求 API、不写 URL。
```

## 实施顺序与验收

1. core 补齐出库状态服务端筛选与物品身份批量投影，更新 OpenAPI、业务文档和定向测试。
2. 前端 API DTO、路由替换、筛选 Dialog、三段式列表、尾部哨兵与详情 Dialog。
3. 用 pending、approved、rejected、指定批次、FIFO、多物品、混合单位和库存不足审批失败数据验收。

必须验证：三种状态库存文案正确；搜索/筛选结果与 `total` 一致；触底加载、失败重试、到底、取消竞争、初始/后台失败和零结果；详情 403/404/网络错误；1440×900、约 768px、390×844 无横向溢出；真实浏览器控制台无新增 error/warning；`pnpm build`、相关 core 测试与 `git diff --check` 通过。

## 非目标

- 不在列表页审批、拒绝、扣库存、编辑、删除或复制出库单。
- 不以前端已加载结果或 `filter-values` 进行伪筛选。
- 不把入库的来源、批次生成或“已入库”文案复制到出库语义。
- 不在前端直接调用 core Rust 函数、读取数据库或访问未受控文件 URL。
