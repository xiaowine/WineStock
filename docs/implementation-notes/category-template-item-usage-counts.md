# 分类与物品属性模板使用数量整改方案

实施状态：已于 2026-07-15 完成。本文保留为跨组件设计与验收记录；实现后的实际 HTTP 契约以运行服务的 OpenAPI 为准。

> 2026-07-26 更新：入库模板概念已整体移除（见 `inbound-template-removal.md`），本文涉及入库模板/默认入库模板的表述仅作历史记录。

## 背景与问题

`/templates` 页面当前可管理物品分类、物品属性模板和入库模板，但前两类记录都没有携带“有多少物品正在使用”的数据。用户在删除分类或物品属性模板前，无法判断影响范围。

这不是单纯的前端展示遗漏：当前 `GET /api/item-categories` 的 `ItemCategoryResponse` 和 `GET /api/item-attribute-templates` 的 `ItemAttributeTemplateResponse` 均没有使用数量字段；两个 `DELETE` 接口仅返回 `204 No Content`。因此前端不能可靠地自行推导数量，页面设计也明确禁止虚构受影响物品数。

数据关系已经存在于 `stock_items`：

- `category_id` 指向物品分类；删除分类只软删除分类，物品保留该 ID。
- `attribute_template_id` 指向物品属性模板；删除模板会清空物品引用，并删除该模板定义的物品属性值。

## 整改目标

1. 分类与物品属性模板列表、详情和删除确认均能显示当前使用它们的物品数量。
2. 数量由 core 在同一批查询中聚合，前端不发起 N+1 请求，也不从分页物品列表猜测。
3. 删除确认在提交完成后获得服务端事务内计算的最终影响数量，用于成功反馈和审计，不以打开 Dialog 时的旧列表数据替代最终结果。
4. 不改变入库模板的含义：它并不被物品直接引用，不能把“被物品属性模板作为默认推荐”伪装成物品使用数量。

## 口径与产品决定

### 列表及详情的 `item_usage_count`

新增非负整数 `item_usage_count`，统一定义为：`stock_items` 中 `deleted_at IS NULL` 且外键字段等于当前分类或物品属性模板 ID 的记录数。

该口径表示“当前有效物品使用数”，与物品目录的可见业务对象一致；软删除物品不计入日常管理的使用数。分类和模板都采用相同口径，避免用户误解同一数字的含义不同。

### 删除响应的 `affected_active_item_count`

分类和物品属性模板删除成功后改为返回 `200 OK` JSON：

```json
{
  "affected_active_item_count": 12
}
```

该数字在删除事务内、写入前计算，口径与 `item_usage_count` 一致，反映本次删除对当前有效物品造成的影响：

- 删除分类：12 个当前物品会保留已失效的分类 ID，需要后续重新归类。
- 删除物品属性模板：12 个当前物品会被解除模板关联，且由模板定义的属性值会被删除。

现有模板删除实现同时会清空所有匹配记录的模板引用。实施时应明确保留这一历史行为，或在同一变更中把更新范围收敛为有效物品；无论选择哪一种，数据库写入范围、`affected_active_item_count` 的口径和删除说明必须保持一致，并补充恢复软删除物品的测试。未做出该决定前，不应把“当前有效物品数量”表述为所有实际被修改记录的数量。

## API 契约

### 分类

`ItemCategoryResponse` 增加：

```ts
item_usage_count: number
```

适用于 `GET /api/item-categories`、`GET /api/item-categories/{id}`、创建和更新响应。

`DELETE /api/item-categories/{id}` 从 `204` 改为 `200`，响应 `CategoryDeleteResponse`：

```ts
interface CategoryDeleteResponse {
  affected_active_item_count: number
}
```

### 物品属性模板

`ItemAttributeTemplateResponse` 增加：

```ts
item_usage_count: number
```

适用于列表、详情、创建、更新和复制响应。

`DELETE /api/item-attribute-templates/{id}` 从 `204` 改为 `200`，响应 `ItemAttributeTemplateDeleteResponse`：

```ts
interface ItemAttributeTemplateDeleteResponse {
  affected_active_item_count: number
}
```

这是公开 HTTP 契约变化。实现前后必须以运行服务的 `/api-docs/openapi.json` 复核，并同步更新 `core/docs/business-api/templates.md`。

## Core 实施设计

### 改动归属与文件级拆分

本需求由 `core` 拥有计数事实和删除结果，由 `frontend` 拥有呈现与交互；前端不得直接查询 SQLite，也不得复用物品目录分页接口来计算计数。

| 层级 | 目标文件 | 具体职责 |
| --- | --- | --- |
| core HTTP DTO | `core/src/stock/controller/templates/categories.rs` | 向 `ItemCategoryResponse` 和新的分类删除响应公开计数。 |
| core HTTP DTO | `core/src/stock/controller/templates/item.rs` | 向 `ItemAttributeTemplateResponse` 和新的模板删除响应公开计数。 |
| core service | `core/src/stock/service/templates/categories.rs` | 将分类仓储投影转换为 HTTP 响应，不能在此追加查询。 |
| core service | `core/src/stock/service/templates/item.rs` | 将模板详情及计数映射为响应；编排删除结果。 |
| core repository | `core/src/persistence/repository/stock_repo/categories.rs` | 查询分类及批量使用数，并在删除事务中返回影响数。 |
| core repository | `core/src/persistence/repository/stock_repo/templates/item.rs` | 查询模板及批量使用数，并在删除事务中返回影响数。 |
| core repository types | `core/src/persistence/repository/stock_repo/types/` 的现有分类/模板类型模块 | 新增内部投影和删除结果类型；不将 controller DTO 下沉到 repository。 |
| frontend API | `frontend/src/api/itemCategories.ts` | 更新分类响应及删除函数返回类型。 |
| frontend API | `frontend/src/api/itemAttributeTemplates.ts` | 更新模板响应及删除函数返回类型。 |
| frontend 页面 | `frontend/src/pages/TemplatesPage.vue` | 在分类/模板列表中显示使用量，接收删除响应并触发成功 Notice。 |
| frontend Dialog | `frontend/src/components/templates/TemplateDeleteDialog.vue` | 接收目标的使用量，呈现差异化影响文案；不直接请求 API。 |
| frontend 样式 | `frontend/src/pages/TemplatesPage.scss` 与模板 Dialog 的现有样式文件 | 在既有三段式行和移动卡片中安排计数，不创建第二套页面骨架。 |

### 内部数据类型

建议新增 repository 内部投影，而不是给 SeaORM entity 增加非表字段：

```rust
pub(crate) struct ItemCategoryWithUsage {
    pub category: stock_item_category::Model,
    pub item_usage_count: u64,
}

pub(crate) struct ItemAttributeTemplateWithUsage {
    pub detail: ItemAttributeTemplateDetail,
    pub item_usage_count: u64,
}

pub(crate) struct TemplateDeletionResult {
    pub affected_active_item_count: u64,
}
```

若分类与模板删除结果需要分别表达，可用两个同字段的业务类型，避免为方便而把分类和模板合成一个语义模糊的“通用模板”类型。HTTP DTO 使用项目现有的稳定数量类型；repository 从 SQLite `COUNT(*)` 读取后必须显式检查不发生窄化。前端使用 `number`，只显示服务端已序列化的非负安全整数。

### 仓储层

- 为分类和物品属性模板定义带 `item_usage_count` 的查询投影，不向 controller 泄漏 SQL 或 SeaORM entity。
- 列表查询使用按关联 ID 聚合的子查询或 `LEFT JOIN` + `COUNT`，一次取回所有记录及计数；计数条件必须包含 `stock_items.deleted_at IS NULL`。
- 单条详情也使用同一投影或复用同一聚合规则，不能出现列表与详情口径漂移。
- 物品属性模板列表现有逐模板读取字段的流程可以保留；使用数量必须先批量聚合为 `template_id -> count` 映射，不能每个模板追加一次 `COUNT`。
- 删除方法在事务中先查询当前有效引用数，再执行既有软删除或模板解绑逻辑，最后返回该计数。找不到有效目标仍返回既有 `404`，不得把零计数与不存在混淆。

建议索引复核：确认 `stock_items(category_id, deleted_at)` 和 `stock_items(attribute_template_id, deleted_at)` 能支撑聚合。若现有单列索引不足，再通过迁移增加复合索引；不要未经 `EXPLAIN QUERY PLAN` 证据盲目创建重复索引。

#### 推荐 SQL 形态

分类列表可使用聚合子查询，避免 `LEFT JOIN` 让后续排序或 ORM 映射产生重复行：

```sql
SELECT categories.*,
       COALESCE(usage.item_usage_count, 0) AS item_usage_count
FROM stock_item_categories AS categories
LEFT JOIN (
  SELECT category_id, COUNT(*) AS item_usage_count
  FROM stock_items
  WHERE deleted_at IS NULL AND category_id IS NOT NULL
  GROUP BY category_id
) AS usage ON usage.category_id = categories.id
WHERE categories.deleted_at IS NULL
ORDER BY categories.sort_order ASC, categories.id ASC;
```

物品属性模板列表必须避免“模板列表一次查询 + 每个模板一次 COUNT”。可先查询所有有效模板及字段，再用一条批量聚合查询获得 `attribute_template_id -> count` 映射：

```sql
SELECT attribute_template_id, COUNT(*) AS item_usage_count
FROM stock_items
WHERE deleted_at IS NULL AND attribute_template_id IS NOT NULL
GROUP BY attribute_template_id;
```

service/repository 再将映射中缺失的 ID 填为零。若为了数据量和结构简化选择单条 join 查询，也必须保留模板字段读取的批量性，不得把字段数和使用数的查询复杂度相乘。

#### 删除事务的精确顺序

以物品属性模板删除为例，推荐顺序如下：

1. 开启现有数据库事务。
2. 查询并确认目标为有效模板；不存在时提交空事务并返回 `None`。
3. 在同一事务内执行 `COUNT(*) FROM stock_items WHERE deleted_at IS NULL AND attribute_template_id = ?`，记录 `affected_active_item_count`。
4. 执行既有模板引用清空、模板定义删除、模板软删除和审计写入。
5. 审计 `details_json` 增加该数量；提交事务。
6. 返回 `Some(TemplateDeletionResult { ... })`，由 service 变为 HTTP `200` JSON。

分类删除采用相同的第 1 至 3、5 至 6 步，但第 4 步只软删除分类本身，不更新 `stock_items.category_id`。这样前端描述“会留下失效分类引用”才与实际行为一致。

SQLite 的单写者事务可以使本次删除的计数与紧随其后的写入保持一致；前端在 Dialog 中显示的列表快照仍可能过时，所以它只能作为预览，最终 Notice 必须读取删除响应。

### Service 与 HTTP 层

- 为分类响应、物品模板响应和两个删除响应增加带中文字段注释的 DTO 字段。
- 将仓储投影映射为 HTTP DTO；禁止在 service 中循环查询使用数量。
- 删除 handler 返回 `Json<...DeleteResponse>` 和 `StatusCode::OK`。
- OpenAPI 注解、错误响应和现有权限保持不变：读取仍为 `stock.template.read`，删除仍为 `stock.template.manage`。
- 审计日志可增加 `affected_active_item_count`，但只记录数值和目标名称，不记录物品名称、属性值或任何敏感数据。

#### OpenAPI 示例

分类列表的一项最终形态：

```json
{
  "id": 7,
  "name": "红葡萄酒",
  "description": "按酒种归类",
  "sort_order": 10,
  "item_usage_count": 24,
  "created_at": "2026-07-15T08:00:00Z",
  "updated_at": "2026-07-15T08:00:00Z"
}
```

删除模板成功响应：

```json
{ "affected_active_item_count": 24 }
```

删除响应不得返回物品 ID 列表。该列表会扩大响应、暴露不必要的物品资料，也会把“删除”接口变成未设计授权和分页的物品检索接口。

## Frontend 实施设计

### API client 与状态

- 在 `itemCategories.ts`、`itemAttributeTemplates.ts` 的响应 DTO 增加 `item_usage_count`。
- 将两个删除函数的返回类型从 `void` 改为相应删除响应 DTO。
- 页面局部更新时保留服务端响应中的计数；创建的初始值应为 `0`，但仍以服务端响应为准。

建议前端删除 API 的明确形态：

```ts
export interface TemplateDeletionResponse {
  affected_active_item_count: number
}

export function deleteItemAttributeTemplate(id: number) {
  return apiClient.request<TemplateDeletionResponse>(
    `/api/item-attribute-templates/${id}`,
    { method: 'DELETE' },
  )
}
```

分类删除使用领域明确的 `ItemCategoryDeletionResponse`，即使当前字段相同也不强制共享前端类型。

### 页面信息设计

#### 桌面列表（1440 × 900）

保留现有三段式行，不新增单独的“使用数”表头列，以免压缩说明和操作区域。

| 域 | 身份段 | 信息段 | 判断与操作段 |
| --- | --- | --- | --- |
| 物品分类 | 名称、`分类 #id` | 描述或“暂无说明” | 第一行 `已用于 N 个物品`；第二行 `排序 n · 更新 HH:mm`；编辑、删除。 |
| 物品属性模板 | 名称、`物品模板 #id`、说明摘要 | `字段 n · 必填 n · 可筛选 n · 目录 n/3` 与默认入库模板 | 第一行 `已用于 N 个物品`；第二行更新时间；查看、编辑、复制、删除。 |

计数使用普通正文与小型物品图标（图标仅作辅助，必须同时显示文字）。`N = 0` 显示“暂未被物品使用”，采用弱化文字但不得使用错误色；`N > 0` 显示“已用于 N 个物品”，使用正文强调色，不用危险色制造误导。

不要把使用数做成排序、筛选或跳转链接。本次范围只让用户理解影响，不新建“按分类/模板查看物品”的页面间导航；该能力应单独确定物品目录查询参数、`stock.item.read` 权限和无权限行为。

#### 移动列表（390 × 844）

- 分类卡片：名称与操作菜单为第一行；说明为第二行；第三行按自然换行展示“已用于 N 个物品 · 排序 n · 更新 HH:mm”。
- 物品模板卡片：名称与状态置顶；字段指标保持现有紧凑键值组；默认入库模板下一行；“已用于 N 个物品”放在更新时间前的元信息行。
- 数字不可被截断为省略号；长中文名称优先换行，按钮最小触控区域和危险操作入口保持现有规范。
- 宽度接近 768px 时，先让信息段换行，再转为卡片布局；禁止通过横向滚动保留桌面三列。

#### 模板详情与分类编辑 Dialog

- 分类编辑 Dialog 在标题下的只读 context 区增加“当前有效物品使用：N 个”；不因编辑分类名或排序而重新请求或预测计数。
- 物品模板只读详情的基础信息区增加“当前有效物品使用：N 个”。编辑态同样可见，但作为只读状态信息，不是表单字段。
- 若列表记录在 Dialog 打开期间被后台刷新，以本次打开时绑定的响应快照展示；保存成功后使用服务端返回的新值替换。不得在编辑草稿中静默改变计数文本。

### 列表与详情

- 分类列表在“排序与操作”信息段显示 `已用于 N 个物品`；数量为零时显示 `暂未使用`，避免把 `0` 误读为加载失败。
- 物品属性模板列表在字段指标后显示 `已用于 N 个物品`；不要把字段数、默认入库模板引用数或属性值条数混入该数字。
- 分类编辑 Dialog 与模板只读详情可显示同一只读使用数，但不把它做成可点击筛选入口；“查看这些物品”属于独立需求，需另行设计物品目录跳转及权限边界。
- 移动端把使用数纳入现有可换行的元信息区，禁止为它新增横向滚动列。

### 删除确认与成功反馈

- 打开确认框时展示列表快照：`当前有 N 个物品使用此分类/模板。` 并明确该数字可能随其他用户操作变化。
- 分类确认文案：删除后这些物品不会自动重新归类。
- 模板确认文案：这些物品将解除模板关联，模板定义的属性值会被删除；保留现有输入模板名称确认。
- 删除成功后使用删除响应的 `affected_active_item_count` 显示 Notice，例如“已删除模板；12 个当前物品已解除模板关联”。不要使用打开 Dialog 时的旧值。
- 若返回 `0`，显示“未影响当前有效物品”，不显示“未影响任何历史数据”。

删除 Dialog 建议增加一个固定的影响提示块，位于业务后果文案之后、操作按钮之前：

```text
影响范围
当前有 12 个有效物品使用此模板。
删除后，这些物品将解除模板关联，其模板定义的属性值将被删除。
```

分类的第三行替换为“删除后，这些物品不会自动重新归类。”当数量为零时保留提示块并显示“当前没有有效物品使用它”，不可直接隐藏，避免用户误以为未加载到数据。

Dialog 的 props 扩充为 `itemUsageCount: number`；`TemplateDeleteTarget` 可直接携带 `itemUsageCount`，避免同一目标名称、ID 与数量分散传递。仅在实际执行删除后，父页面根据 API 响应生成 Notice；Dialog 不负责成功 toast，也不假定删除结果。

### 前端状态流转

```text
列表已加载
  -> 打开删除 Dialog（读取行快照 item_usage_count）
  -> 用户取消：不改列表
  -> 用户确认：仅锁定该 Dialog，按钮显示“正在删除…”
  -> DELETE 200：从本地域列表移除目标，关闭 Dialog，按响应数显示 Notice
  -> DELETE 403：保留 Dialog 和草稿，显示权限变化错误，并按现有策略刷新域数据
  -> DELETE 404：关闭 Dialog，提示记录已被删除，刷新当前域
  -> 网络失败：保留 Dialog、名称确认输入和行快照，允许重试
```

对于模板名称确认，任何错误、重试或网络失败都不得清空用户已输入的确认名称；只有成功关闭或用户主动关闭后才清理。

## 分阶段实施顺序

1. 先确认模板删除对软删除物品的最终写入策略，并在方案评审中冻结口径。
2. 实现 core 仓储聚合、事务内删除计数、DTO、handler 和 OpenAPI。
3. 编写 core 集成测试，确认 API 和实际删除副作用一致。
4. 更新前端 API DTO、页面列表、详情、删除 Dialog 与 Notice。
5. 更新 API/页面文档和代码地图；运行定向构建、测试与真实浏览器检查。

## 验收与测试

### Core

- 分类和模板列表、详情能分别返回 0、1 和多个有效物品的正确计数。
- 软删除物品不计入 `item_usage_count`。
- 更换物品分类或模板后，旧记录计数递减、新记录递增。
- 删除分类返回准确的 `affected_active_item_count`，物品仍保存旧分类 ID。
- 删除物品属性模板返回准确的 `affected_active_item_count`，并验证模板引用、定义和属性值的实际处理结果。
- 不存在或已删除目标保持 `404`；无权限保持 `403`。
- 用至少多条分类/模板和物品数据验证列表查询不出现按记录数量增长的 SQL 查询。

### Frontend

- 在桌面 `1440 × 900`、断点附近和移动 `390 × 844` 检查数量文本对齐、换行和无横向溢出。
- 覆盖首次加载、后台刷新、空列表、零使用数、多个使用数、删除中、删除成功、`403`、`404` 和网络失败保留草稿的状态。
- 使用浏览器开发者工具检查删除请求按新 `200` JSON 契约解析，控制台没有新增 error、warning 或 issue。
- 运行 `pnpm build`；core 使用覆盖本次 API 与仓储路径的最窄 `cargo +stable test -p winestock-core <相关测试筛选>`，再按公共 HTTP 契约变更的需要扩大验证。

## 文档与代码地图同步

- `core/docs/business-api/templates.md`：字段、删除响应和口径。
- `frontend/docs/page-templates.md`：列表指标、删除确认和异步反馈。
- `docs/code-map/core.md` 与其 stock/http API 子地图：新增查询投影和删除响应职责。
- `docs/code-map/frontend.md`：更新模板 API client 与页面职责说明。

本文件为跨组件实施方案，不在未经实现和验收前宣称接口已经提供使用数量。
