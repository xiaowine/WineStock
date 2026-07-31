# SQLite RETURNING 整改方案

> 状态：两批选择性整改已完成
> 日期：2026-07-29
> 所属范围：`core` SQLite 持久化与工作区 SeaORM feature

## 目标

在保持数据库 schema、HTTP 契约、业务权限和事务语义不变的前提下，正式拥有 SeaORM 的 SQLite
`RETURNING` 能力，并只迁移能够减少数据库查询、且返回语义与现有代码等价的写入路径。

本整改不以“所有写入都使用 RETURNING”为目标。没有返回需求的写入继续使用 `execute_raw`；返回模型依赖
JOIN、聚合或子记录加载时，继续在事务内或提交后执行现有查询。

## 整改前基线

- `sea-orm-migration 2.0.0` 通过 SeaORM defaults 传递开启
  `sqlite-use-returning-for-3_35`，但工作区尚未直接声明，能力所有权不稳定。
- `core` 使用 SQLx bundled SQLite，连接测试已经执行 `SELECT sqlite_version()` 并要求版本不低于 3.35。
- 现有生产 SQL 没有 `RETURNING`，主要通过 `ExecResult::last_insert_id()` 获取自增主键，再按需要查询读取模型。

## 候选路径

| 路径 | 现有行为 | 决策 | 原因 |
|---|---|---|---|
| 创建库位分组 | INSERT 后按 ID SELECT | 迁移 | 单表读取模型可由 INSERT RETURNING 完整返回，减少一次查询 |
| 更新库位分组 | UPDATE 后按 ID SELECT | 迁移 | 单表读取模型可由 UPDATE RETURNING 完整返回，减少一次查询 |
| 创建移库记录 | INSERT 后提交，再按 ID SELECT | 迁移 | 单表读取模型可完整返回，避免提交后的额外查询 |
| 创建 JWT 签名密钥 | Entity INSERT 后按 ID SELECT | 迁移 | `exec_with_returning` 可直接返回完整 Model |
| 创建刷新令牌 | Entity INSERT 后按 ID SELECT | 迁移 | 登录与刷新路径每次减少一次查询 |
| 创建用户 | Entity INSERT 后按 ID SELECT | 迁移 | 新记录必为 active，直接返回与原过滤查询等价 |
| 创建文件元数据 | Entity INSERT 后按 ID SELECT | 迁移 | 单表 Model 可直接返回，并删除专用回查方法 |
| 创建物品分类 | Entity INSERT 后按 ID SELECT | 迁移 | 返回 Model 继续用于同一事务内审计 |
| 创建物品 | Entity INSERT 后按 ID SELECT | 迁移 | 创建接口此处只需要单表 Model；属性在随后独立写入 |
| 创建属性模板主记录 | Entity INSERT 后按 ID SELECT | 迁移 | 主记录 Model 可直接返回，字段明细仍按原事务写入 |
| 创建/更新库位 | 写入后 JOIN 分组查询 | 保留 | `group_name` 不属于 `stock_locations`，单表 RETURNING 不能提供等价模型 |
| 创建入库/出库订单 | 主键用于写入明细，最终加载订单与明细 | 保留 | `RETURNING id` 相比 `last_insert_id()` 不减少查询 |
| 创建批次、默认库位分组、私有属性定义 | 只需要主键或影响行数 | 保留 | 主键直接用于后续明细或文件绑定，没有额外 SELECT 可消除 |
| DELETE 与软删除 | 调用方只需要是否成功或影响行数 | 保留 | 返回整行没有业务消费者 |

## 实施设计

1. 在工作区直接 SeaORM feature 中加入 `sqlite-use-returning-for-3_35`，不再依赖 Migration 的传递 defaults。
2. 手写 SQL 使用 `INSERT ... RETURNING ...` 或 `UPDATE ... RETURNING ...`，通过
   `query_one_raw(Statement::from_sql_and_values(...))` 读取单行结果。
3. 返回列显式列举，不使用 `RETURNING *`；继续复用现有 `QueryResult -> Record` 映射函数。
4. 预期必须命中一行的写入若没有返回结果，转换为 `DbErr::RecordNotFound`，不静默构造默认模型。
5. 所有审计事件继续与业务写入处于同一事务；审计快照使用 RETURNING 得到的数据库实际值。
6. 已有 Entity INSERT 路径使用 SeaORM `exec_with_returning`；保留手写 SQL 的路径继续使用 raw API，
   不为迁移 RETURNING 强行改写为 Entity，也不引入 `RestrictedConnection`。

## 测试与验收

- SQLite 实际版本不低于 3.35。
- 创建库位分组返回数据库生成 ID 与完整字段；同名冲突仍失败且不产生审计事件。
- 更新库位分组返回更新后的字段；不存在或已删除记录仍保持原有响应语义。
- 移库记录返回完整字段，库存变更、审计写入和事务回滚语义保持不变。
- core 全量测试、workspace 全量测试、server Release、Android ARM64 native 与 Debug/Release APK 构建通过。
- 依赖树显示该 feature 同时有工作区直接声明与 Migration 传递来源；以后移除 Migration defaults 时能力不丢失。

## 回退条件

出现返回列解码差异、冲突错误映射变化、事务内审计快照变化、Android bundled SQLite 不兼容或产物构建失败时，
回退具体生产 SQL 到原有“execute + 查询”路径；SQLite 3.35 最低版本承诺和显式 feature 是否保留需按失败原因单独判断。

## 实施结果

- 工作区已直接声明 `sqlite-use-returning-for-3_35`；依赖树同时显示项目直接来源和 SeaORM Migration defaults 传递来源。
- 库位分组创建由“INSERT + SELECT”改为 `INSERT ... RETURNING`，减少一次事务内查询。
- 库位分组更新由“UPDATE + SELECT”改为 `UPDATE ... RETURNING`，减少一次事务内查询。
- 移库记录创建由“INSERT + 提交后 SELECT”改为 `INSERT ... RETURNING`，删除了仅服务于该流程的查询方法。
- JWT 签名密钥、刷新令牌、用户、文件元数据、物品分类、物品和属性模板主记录改用 SeaORM
  `exec_with_returning`，共消除七次按主键回查；ActiveModel 字段、事务和审计流程保持不变。
- 文件仓储中只服务于创建回查的 `find_by_id` 已删除。
- 新增底层事务测试：先取得 RETURNING 行，再制造唯一约束冲突并回滚，确认返回行不代表已经提交。
- 扩展库位业务测试，覆盖创建/更新返回字段、重复创建不增加审计、移库完整返回、库存变化和审计事件。

2026-07-29 已通过：

```text
cargo +stable test -p winestock-core --locked                         # 122 passed
cargo +stable check --workspace --all-targets --locked
cargo +stable test --workspace --locked                               # 146 passed
cargo +stable build -p winestock-server --release --locked
cargo ndk -t arm64-v8a -P 28 check -p winestock-android-native --locked
gradlew.bat :app:testDebugUnitTest :app:assembleDebug --no-daemon --no-parallel
gradlew.bat :app:assembleRelease --no-daemon --no-parallel
```

Gradle 同时通过 Debug/Release native library、ELF/JNI/ABI、前端资源及最终 APK 包级校验。真实 Android
设备上的 SQLite 版本读取和十条业务路径 smoke 仍属于发布前真机验收，不在本次主机构建结果中虚报覆盖。
