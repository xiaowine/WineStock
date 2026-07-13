# 统一物品属性定义实施方案

本文记录物品模板属性与物品自定义属性统一使用同一种属性定义实体的确认方案。
本方案同时涉及 `core` 数据库、repository、service、HTTP 契约和 `frontend` 物品编辑流程，因此放在根级跨组件实施记录中。

状态：已于 2026-07-13 完成实施。最终结构已同步到数据库文档、业务 API 文档、校验文档和前后端代码地图；本文继续保留为跨组件决策与数据库转换记录。

## 目标

改造前模板字段由 `stock_item_attribute_template_fields` 保存，自定义属性则把字段名称和类型直接保存在 `stock_item_attributes` 中。
两类属性值虽然共用值表，但字段定义仍存在两套来源语义。

目标调整为：

- 模板属性和自定义属性共用 `stock_item_attribute_definitions`。
- 模板只是多个模板属性定义的组合，不增加模板与定义的多对多关联表。
- 模板定义只属于一个模板；不同模板中的同名属性各自拥有独立定义。
- 自定义定义只属于一个物品，一个物品可以拥有多个私有自定义定义。
- `stock_item_attributes` 只保存物品、定义引用、实际值、实际单位和排序。
- 当前处于开发阶段，不保留旧表名、旧字段名、旧 HTTP 字段或历史兼容 migration。

## 目标数据库结构

### `stock_item_attribute_definitions`

统一保存模板属性定义和物品私有自定义定义：

```text
id
template_id            nullable
owner_item_id          nullable
field_name
field_type
required
searchable
options_json
default_value
unit_mode
fixed_unit
unit_options_json
sort_order
created_at
updated_at
```

归属必须且只能存在一种：

```sql
CHECK (
    (template_id IS NOT NULL AND owner_item_id IS NULL)
    OR
    (template_id IS NULL AND owner_item_id IS NOT NULL)
)
```

当前不支持无归属的全局公共属性定义，也不支持定义同时属于模板和物品。

建议索引和唯一性：

- 模板定义在同一 `template_id` 内按忽略大小写的 `field_name` 唯一。
- 私有定义在同一 `owner_item_id` 内按忽略大小写的 `field_name` 唯一。
- `template_id`、`sort_order`、`id` 支持模板字段顺序查询。
- `owner_item_id`、`sort_order`、`id` 支持物品私有定义顺序查询。

### `stock_item_attributes`

物品属性值表调整为：

```text
id
item_id
definition_id          NOT NULL
value_json
unit
sort_order
created_at
updated_at
```

约束：

- `(item_id, definition_id)` 唯一。
- `definition_id` 外键指向 `stock_item_attribute_definitions`。
- 删除属性定义时级联删除引用它的属性值。
- 删除属性值时，现有图片绑定按外键级联删除；失去引用的文件继续使用项目已有文件清理策略。

字段名称、字段类型和单位规则不再复制到属性值表。

## 生命周期规则

### 模板定义

- 两个模板即使包含同名、同类型属性，也创建独立定义 ID。
- 复制模板时，为复制结果创建一组新的独立定义。
- 编辑模板时，已有字段必须提交 `definition_id`，并原位更新定义。
- 新模板字段不提交 `definition_id`，由服务端创建定义。
- 请求中未再提交的已有字段视为删除。
- 删除模板字段时直接删除定义，并级联删除所有物品对应的属性值，不保留历史。
- 删除仍被物品使用的模板是允许的：清空物品的 `attribute_template_id`，删除模板定义和对应值，保留物品及其自定义属性。

### 自定义定义

- 新增自定义属性时创建 `owner_item_id` 指向当前物品的私有定义。
- 同一物品可以拥有多个私有定义。
- 已有自定义属性保存时提交 `definition_id`，普通编辑原位更新定义并保持 ID 稳定。
- 新自定义属性不提交 `definition_id`，由服务端在物品事务内创建。
- 删除自定义属性时同时删除属性值和私有定义。
- 删除物品时删除该物品的属性值和私有定义，但不得删除模板定义。
- 本次不支持把私有自定义定义提升为模板定义。

### 模板切换

- 删除原模板定义对应的物品属性值。
- 保留该物品的全部私有自定义属性。
- 根据新模板字段创建或补齐模板属性值。
- 如果私有自定义属性与目标模板字段忽略大小写重名，拒绝模板切换。
- 前端应在提交前检测冲突，服务端仍执行相同校验作为最终边界。
- 不自动删除、重命名或转换冲突的自定义属性。

## 字段和值规则

### 唯一性

同一个物品最终呈现的全部属性名称忽略大小写唯一，包括模板属性与自定义属性之间的组合。

### 必填规则

- 自定义属性的名称、类型和值提交时均不能为空。
- 模板属性继续服从定义中的 `required`。
- `required = false` 的模板属性允许无值，并且不应为了提交而创建无意义的空属性值。

### 字段类型

继续支持：

```text
text
number
select
date
file
url
boolean
```

自定义 `select` 必须提供至少一个非空、裁剪后忽略大小写不重复的候选项，实际值必须来自候选项。

### 单位规则

只有 `number` 类型允许配置单位规则：

```text
none
fixed
select
```

其它类型固定使用 `none`，类型从 `number` 变更为其它类型时清除单位规则和实际单位。

自定义属性与模板属性使用相同的单位规则校验：

- `none`：不保存实际单位。
- `fixed`：定义必须提供固定单位，实际值使用定义单位。
- `select`：定义必须提供非空、去重候选单位，实际单位必须来自候选项。
- `fixed` 在界面中显示为“指定单位”，定义必须提供单位值，实际值由服务端派生。

### 搜索

- 模板定义只有 `searchable = true` 时参与结构化属性筛选和筛选值聚合。
- 物品私有自定义定义固定 `searchable = false`，前端不展示该配置。
- 自定义属性名称和值仍可参与普通关键词搜索。

## HTTP 契约调整

开发阶段直接进行破坏性字段重命名：

```text
template_field_id -> definition_id
```

不保留旧字段兼容。

### 模板字段请求

已有字段：

```json
{
  "definition_id": 12,
  "field_name": "容量",
  "field_type": "number"
}
```

新字段不提交 `definition_id`。

服务端必须验证已有定义属于当前模板，不能跨模板修改定义。

### 物品属性请求

模板属性：

- 必须提交目标模板中的 `definition_id`。
- 字段名称、类型和单位规则以服务端定义为准。

已有自定义属性：

- 提交私有 `definition_id`。
- 可同时提交字段名称、类型、候选项和单位规则更新。
- 服务端必须确认定义的 `owner_item_id` 等于当前物品 ID。

新自定义属性：

- 不提交 `definition_id`。
- 提交完整定义和实际值。
- 服务端在同一物品保存事务中创建私有定义和值。

物品属性响应统一返回 `definition_id` 和完整定义投影，不再返回 `template_field_id`。

## Core 实施范围

### 初始 schema

- 将 `stock_item_attribute_template_fields` 重命名为 `stock_item_attribute_definitions`。
- 增加 `owner_item_id`，并把 `template_id` 改为可空。
- 增加归属互斥检查、两组局部唯一索引和查询索引。
- 重建 `stock_item_attributes`，使用非空 `definition_id`，删除重复字段定义列。
- 更新建表顺序、外键和反向 DROP 顺序。
- 不新增兼容 migration，直接修改 `m20260706_000001_initial_schema.rs`。

### Entity 和 repository

- `item_attribute_template_field.rs` 重命名为 `item_attribute_definition.rs`。
- 更新 entity 注册、仓储类型和所有查询引用。
- 模板仓储按 `template_id` 管理模板定义。
- 物品仓储在单一事务中管理私有定义、属性值、图片绑定和审计记录。
- 模板字段删除、模板删除和物品删除必须在事务内完成级联清理。
- 搜索查询改为连接统一定义表。

### Service 和 controller

- DTO 使用 `definition_id`。
- 模板字段更新根据 ID 区分新增、更新和删除。
- 物品属性归一化区分模板定义、已有私有定义和新私有定义。
- 验证定义归属、名称唯一、类型化值、候选项、单位规则和文件所有权。
- 模板切换执行重名检测并返回可定位的字段错误。
- 移除删除已使用模板的旧阻止逻辑。
- 同步更新 OpenAPI schema、HTTP 测试和业务文档。

## Frontend 实施范围

### API 与草稿

- TypeScript DTO 将 `template_field_id` 改为 `definition_id`。
- 草稿字段将 `templateFieldId` 改为 `definitionId`，并记录定义归属和自定义定义配置。
- 已有模板定义和私有定义保存时保持 ID；新增自定义定义不提交 ID。
- 请求差异比较和草稿指纹包含候选项及单位规则。

### 自定义属性编辑

新增自定义属性默认值：

```text
名称：空
类型：text
值：空
单位规则：none
可搜索：false
```

渐进展示：

- `select` 显示可增删候选项列表。
- `number` 主表单显示固定高度的单位摘要和设置入口；单位模式及候选项在独立 Dialog 中编辑，实际单位选择器与属性值同行。
- `select` 单位模式显示可增删单位候选项列表。
- 其它类型不显示单位配置。

候选项规则：

- 裁剪首尾空格。
- 不允许空候选项。
- 忽略大小写去重。
- 删除当前选中候选项时清空实际值。

类型变化：

- 当前值非空时先确认。
- 确认后清空原值。
- 从 `file` 离开时清理临时图片。
- 清理不兼容的候选项、单位规则和实际单位。
- 不进行隐式值转换。

### 模板切换

- 保留私有自定义属性。
- 删除旧模板属性草稿。
- 在应用目标模板前检测名称冲突。
- 冲突时保持原草稿不变，并列出冲突属性名称。
- 服务端错误仍需映射为同等明确反馈。

## 测试要求

### Core

- 初始 schema 只创建新表名和新字段。
- 定义归属互斥检查。
- 同模板和同物品字段名称忽略大小写唯一。
- 模板字段新增、原位更新和删除。
- 模板复制产生独立定义 ID。
- 删除模板字段级联删除属性值和图片绑定。
- 删除使用中的模板清空物品模板 ID，并保留私有定义。
- 物品可创建多个自定义定义。
- 自定义定义原位更新并保持 ID。
- 删除自定义属性删除私有定义。
- 删除物品只删除私有定义。
- 模板切换保留自定义属性并拒绝名称冲突。
- 自定义 select、单位规则、必填值和文件所有权校验。
- OpenAPI 不再包含 `template_field_id`。

### Frontend

- DTO、草稿和请求只使用 `definition_id`。
- 新建自定义属性不能以空名称或空值提交。
- 类型变化确认与清理。
- select 候选项增删、去重和当前值清理。
- number 单位规则编辑。
- 模板切换保留自定义属性。
- 模板切换重名时不修改草稿。
- 已有私有定义保存后 ID 稳定。
- 桌面和移动端检查字段布局、溢出、Dialog 滚动和控制台。

## 当前测试数据库转换

用户要求保留 `target/debug/data/winestock.sqlite` 中现有模板、物品和属性值。
代码完成并通过新库测试后，使用 `sqlite3` 在事务中原地转换。

转换顺序：

1. 停止 WineStock server，并确认没有进程占用数据库。
2. 备份数据库文件。
3. 暂时关闭外键检查并开始独占事务。
4. 创建新的 `stock_item_attribute_definitions`。
5. 把旧模板字段复制为 `template_id` 有值的模板定义，并保留原 ID。
6. 对每条旧自定义属性创建 `owner_item_id` 指向所属物品的私有定义。
7. 创建新的 `stock_item_attributes`，模板属性沿用旧字段 ID，自定义属性回填新定义 ID。
8. 重建 `storage_item_file_bindings` 外键引用所需结构，保留属性值和文件绑定关系。
9. 删除旧表并重命名新表。
10. 重建索引，恢复外键检查并提交事务。
11. 执行 `PRAGMA foreign_key_check`。
12. 对比转换前后的模板数、定义数、物品数、属性值数和文件绑定数。
13. 修正 `seaql_migrations`，只保留当前初始 schema 版本记录。
14. 启动 server，检查 migration、OpenAPI、物品列表和物品编辑 smoke 流程。

若转换前发现同一物品内存在忽略大小写重名属性、无效 select 候选值或其它不满足新约束的数据，必须停止事务并报告冲突，不得静默删除或改名。

## 实施顺序

1. 修改初始 schema 和 entity，并让新库 migration 测试通过。
2. 修改 repository 和事务行为。
3. 修改 service、controller、OpenAPI 与 core 测试。
4. 修改前端 DTO、草稿、编辑器和模板切换行为。
5. 更新正式文档和代码地图。
6. 执行 Rust 格式、check、相关测试和前端生产构建。
7. 使用 SQLite 转换当前测试数据库。
8. 执行真实 server 启动、OpenAPI 和前端 smoke 检查。

实施期间不得只改表名而保留旧语义，也不得为了通过测试引入 `template_field_id` 兼容层或第二套自定义属性结构。
