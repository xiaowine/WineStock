# 替代料独立业务域迁移实现文档

本文档记录替代料 API 从物品子路径迁移为独立业务域的实施方案。

## 背景

替代料关系本质上仍然是“一个库存物品可由哪些库存物品替代”，但前端和权限使用者看到的是独立的替代料管理能力，不应长期暴露为物品管理的子接口。

当前实现还残留在 `items` 路径和 `bind` 命名中：

- `POST /api/items/{id}/substitutes`
- `GET /api/items/substitutes`
- `GET /api/items/{id}/substitutes`
- `DELETE /api/items/{id}/substitutes/{substitute_id}`
- `SubstituteBindRequest`
- `bind_substitutes`
- `BindStockSubstitute`

本次迁移不保留旧 URL 兼容层。实现完成后，旧路径应从路由、OpenAPI、业务文档和测试中全部移除。

## 目标

- 替代料管理作为独立业务域暴露在 `/api/substitutes` 下。
- 接口动作使用“整体替换”和“删除关系”的真实语义，不再使用 `bind`。
- 路径参数使用 `item_id` 和 `substitute_item_id`，避免把替代物品 ID 误解为替代关系 ID。
- 不改变底层业务规则、权限和数据库结构。
- 不保留旧路径、旧 handler 或旧 DTO 的兼容包装。

## 目标 API

| 方法       | 路径                                                | 说明             | 权限                        |
|----------|---------------------------------------------------|----------------|---------------------------|
| `GET`    | `/api/substitutes`                                | 查询全部替代料关系      | `stock.substitute.read`   |
| `GET`    | `/api/substitutes/{item_id}`                      | 查询指定物品的替代料列表   | `stock.substitute.read`   |
| `PUT`    | `/api/substitutes/{item_id}`                      | 整体替换指定物品的替代料列表 | `stock.substitute.manage` |
| `DELETE` | `/api/substitutes/{item_id}/{substitute_item_id}` | 删除单个替代料关系      | `stock.substitute.manage` |

旧路径映射如下，实施时只迁移到新路径，不保留旧路径：

| 旧接口                                                  | 新接口                                                      |
|------------------------------------------------------|----------------------------------------------------------|
| `GET /api/items/substitutes`                         | `GET /api/substitutes`                                   |
| `GET /api/items/{id}/substitutes`                    | `GET /api/substitutes/{item_id}`                         |
| `POST /api/items/{id}/substitutes`                   | `PUT /api/substitutes/{item_id}`                         |
| `DELETE /api/items/{id}/substitutes/{substitute_id}` | `DELETE /api/substitutes/{item_id}/{substitute_item_id}` |

## 保持不变的业务语义

- 替代料关系仍然以库存物品为主体，不迁移到模板字段或模板元数据。
- `PUT /api/substitutes/{item_id}` 是整体替换语义：请求体中的列表成为该物品最新完整替代料列表。
- `substitutes: []` 仍表示清空该物品所有替代料关系。
- 校验规则保持不变：禁止自引用、重复替代物品、重复优先级和循环替代关系。
- 查询结果仍只返回未软删除的主物品和未软删除的替代物品。
- 权限代码保持 `stock.substitute.read` 和 `stock.substitute.manage`。
- 数据库表 `stock_substitutes` 和迁移脚本保持不变。

## 可观察变化

- 旧的 `/api/items/.../substitutes` 路径全部失效。
- OpenAPI 只发布新的 `/api/substitutes...` 路径。
- `POST` 替换为 `PUT`，因为该接口是整体替换目标资源的当前列表。
- 替代料 OpenAPI tag 应从泛化的 `stock` 调整为 `substitutes`。
- 本次落地不调整审计动作：整体替换仍写 `linked`，单条删除仍写 `unlinked`。原因是 `audit_events.action` 当前有数据库 `CHECK` 约束，且本方案目标是不改变数据库结构。

## 代码改名清单

### HTTP controller

文件：`core/src/stock/controller/substitutes.rs`

| 当前名称                       | 目标名称                         |
|----------------------------|------------------------------|
| `SubstituteItem`           | `SubstituteReplacementItem`  |
| `SubstituteBindRequest`    | `SubstituteReplaceRequest`   |
| `SubstituteDetailResponse` | `ItemSubstituteResponse`     |
| `bind_substitutes`         | `replace_substitutes`        |
| `list_all_substitutes`     | `list_substitute_relations`  |
| `list_substitutes`         | `list_item_substitutes`      |
| `delete_substitute`        | `delete_substitute_relation` |

处理函数的 `#[utoipa::path]` 需要同步改为新路径、新方法、新 tag 和新参数名。

### Service

文件：`core/src/stock/service/substitutes.rs`

服务函数与 controller 对齐：

- `replace_substitutes`
- `list_substitute_relations`
- `list_item_substitutes`
- `delete_substitute_relation`

服务注释需要明确：替代料是独立 API 业务域，但仍依赖库存物品存在性和库存仓储。

### Repository

文件：

- `core/src/persistence/repository/mod.rs`
- `core/src/persistence/repository/stock_repo/types.rs`
- `core/src/persistence/repository/stock_repo/substitutes.rs`

建议把 `BindStockSubstitute` 改为 `StockSubstituteInput`，表示替代料关系写库输入，而不是绑定动作。

`StockRepository::replace_substitutes()` 名称可以保留，因为它已经表达整体替换语义。内部审计动作如果改为 `replaced`
，需要同步测试和事件文档。

### Router 和 OpenAPI

文件：

- `core/src/stock/mod.rs`
- `core/src/stock/controller.rs`
- `core/src/stock/service.rs`
- `core/src/http/docs.rs`

路由应收敛为：

```text
/substitutes
/substitutes/{item_id}
/substitutes/{item_id}/{substitute_item_id}
```

删除所有 `/items/{id}/substitutes` 和 `/items/substitutes` 路由注册。

OpenAPI schema 引用、path 收集和 tag 描述需要使用新 DTO 与新 handler 名称。

## 测试改造

文件：`core/src/tests/stock_substitutes.rs`

需要覆盖：

- `PUT /api/substitutes/{item_id}` 可整体替换替代料列表。
- `PUT /api/substitutes/{item_id}` 传空列表可清空替代料。
- `GET /api/substitutes/{item_id}` 返回指定物品替代料。
- `GET /api/substitutes` 返回全量替代料关系。
- `DELETE /api/substitutes/{item_id}/{substitute_item_id}` 删除单条关系。
- 自引用、重复替代物品、重复优先级和循环绑定仍返回 `400 invalid_request`。
- 缺失 `stock.substitute.read` 或 `stock.substitute.manage` 时仍返回 `403`。
- 旧路径不再注册，可增加轻量断言确认旧 `/api/items/.../substitutes` 返回未匹配路由结果。

## 文档改造

需要同步更新：

- `core/docs/business-api/substitutes.md`
- `core/docs/business-api/permissions.md`
- `core/docs/business-api.md`
- `docs/code-map.md`
- `core/docs/validation/core-src-stock-controller.md`
- `core/docs/validation/core-src-persistence-repository-stock-repo.md`
- 如审计动作改为 `replaced`，同步更新 `core/docs/business-api/events.md` 和相关测试预期。

如果 `core/docs/business-api/implementation-order.md` 提到旧路径，也要一并更新。

## 清理检查

实施完成后运行以下搜索，结果中不应再出现旧路径或旧命名：

```text
rg -n "/items/.+substitutes|items/substitutes|SubstituteBindRequest|bind_substitutes|BindStockSubstitute|substitute_id" core docs --glob "!core/docs/implementation-notes/substitute-api-domain-migration-plan.md"
```

其中 `substitute_item_id` 是目标字段名，可以保留；裸 `substitute_id` 不应再作为路径参数或函数参数出现。

## 验收标准

- OpenAPI JSON 只包含 `/api/substitutes`、`/api/substitutes/{item_id}` 和
  `/api/substitutes/{item_id}/{substitute_item_id}`。
- 旧 `/api/items/.../substitutes` 路径不存在。
- 替代料权限仍为 `stock.substitute.read` 和 `stock.substitute.manage`。
- 底层 `stock_substitutes` 表结构不变，不需要数据库迁移。
- 相关源码注释和文档说明为中文，且不再描述旧路径。
- `cargo +stable fmt --all -- --check` 通过。
- `cargo +stable check --workspace --all-targets` 通过。
- `cargo +stable test --workspace` 通过。
- `cargo +stable build -p winestock-server` 通过。
