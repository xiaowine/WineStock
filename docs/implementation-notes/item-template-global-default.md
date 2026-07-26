# 全局默认物品属性模板实施方案（2026-07-27，同日实施完成）

> 实施状态：core（含测试）、契约同步、前端接线与模板页 UI 已完成；桌面与 Android 存量测试库
> 已按下文原地转换。同批附带用户要求的调整：模板删除确认取消“输入模板名称验证”，改为
> 普通的后果说明 + 直接确认。

延续 2026-07-26 库存域讨论中"全局默认"方向的第一项落地：为物品属性模板提供全站唯一默认，
新建物品免去每次手选模板。本次只做模板默认；分类与库位默认按同一模式后续单独实施。

## 决策要点

- **全局非每用户**：默认是站点级配置，所有账号一致；
- **至多一个默认**：服务层事务保证，设置新默认自动清除旧默认，不依赖前端；
- **仅新建预填**：默认模板在新建物品草稿创建时预选并应用字段（等同用户手选），
  用户可换成其它模板或"不使用模板"；编辑已有物品不受影响；
- **未设置默认时行为与现状一致**（不使用模板），向后完全兼容；
- **删除默认模板随删清空**，不迁移默认到其它模板；复制模板不继承默认标记。

## 数据模型（不新建迁移）

按用户要求直接修改现有初始迁移 `m20260706_000001_initial_schema.rs`：

```sql
-- stock_item_attribute_templates 增列
is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1))
```

“至多一个默认”不建唯一索引（软删除语义下部分索引表达繁琐），由服务层事务保证：
设置默认时先 `UPDATE ... SET is_default = 0 WHERE is_default = 1`，再置目标为 1。

**存量测试库原地转换**（迁移已应用过的库不会重放）：

```sql
ALTER TABLE stock_item_attribute_templates ADD COLUMN is_default INTEGER NOT NULL DEFAULT 0;
```

需转换：桌面测试库（`server` 数据目录）与 Android 测试机
（`no_backup/winestock/data/winestock.sqlite`，经 adb 停止应用后执行）。

## core 改动

| 位置                                                       | 内容                                                                                                                                                                                     |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `persistence/migration/m20260706_000001_initial_schema.rs` | 表定义增列                                                                                                                                                                               |
| `persistence/entity/item_attribute_template.rs`            | Model 增 `is_default: bool`（SQLite INTEGER 0/1）                                                                                                                                        |
| `persistence/repository/stock_repo/templates/item.rs`      | `update_item_attribute_template` 支持 `is_default`：置 1 前事务内清除其它默认；软删除默认模板时该行随软删除自然失效（`is_default` 保留但查询按 `deleted_at IS NULL` 过滤，无需额外清理） |
| `persistence/repository`（Update 输入结构）                | `UpdateItemAttributeTemplate` 增 `is_default: Option<bool>`                                                                                                                              |
| `stock/controller/templates/item.rs`                       | `ItemAttributeTemplateUpdateRequest` 增可选 `is_default`；`ItemAttributeTemplateResponse` 增 `is_default: bool`                                                                          |
| `stock/service/templates/item.rs` + `common.rs`            | 透传与响应映射                                                                                                                                                                           |
| `tests/stock_attribute_templates.rs`                       | 新增：设默认、换默认自动清旧、响应携带、删除默认后列表无默认                                                                                                                             |

不加独立"设为默认"端点：走既有 `PUT /api/item-attribute-templates/{id}`，改动面最小，
权限沿用 `stock.template.manage`。

## 契约同步

core 改完后执行 `cd frontend && pnpm gen:api`（dump_openapi + 类型再生成），
`ItemAttributeTemplateResponse` 自动获得 `is_default`。

## frontend 改动

| 位置                                            | 内容                                                                                                                                               |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pages/items/model.ts`                          | 新增 `defaultAttributeTemplate(templates)` 取默认模板的纯函数；新建草稿装配处调用                                                                  |
| `ItemsPage` 新建会话 + `useItemCreateSession`   | 模板元数据加载完成且草稿仍为初始态（未选模板、无属性、指纹未变）时，预选并 `applyAttributeTemplate` 默认模板，同步刷新基线指纹避免"未保存更改"误报 |
| `LcscItemLookupDialog` / `LcscLookupFlowDialog` | 确认面板模板预选从 `templates[0]` 改为 默认模板 ?? 第一项                                                                                          |
| `TemplatesPage`（物品属性模板 tab）             | 行内"设为默认"操作与"默认"标记；设默认调用更新接口 `{ is_default: true }`；取消默认 `{ is_default: false }`                                        |
| 文档                                            | `page-templates.md`、`page-items.md`、code-map 相应条目                                                                                            |

## 验证计划

1. `cargo test -p winestock-core`（模板域用例）；
2. `pnpm gen:api` 后 `vue-tsc` 与前端测试套件；
3. 浏览器：模板页设默认/换默认/取消默认与标记显示；新建物品预选并已应用字段；
   立创确认面板预选默认；未设默认时行为回归现状；
4. 存量库 ALTER 后桌面与 Android 实测。

## 明确不做

- 分类、库位默认（同模式后续单独任务）；
- 每用户默认、默认模板锁定（用户始终可改选）；
- 删除默认时自动指定新默认。
