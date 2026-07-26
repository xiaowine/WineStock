# 业务权限代码汇总

本业务 API 引入的权限代码：

| 权限代码 | 所属模块 | 说明 |
|----------|----------|------|
| `stock.read` | 历史兼容 | 历史兼容的库存只读权限；具体查询接口使用细分权限 |
| `stock.write` | （已有） | 创建或修改库存数据 |
| `stock.item.read` | 物品管理 | 查看物品列表、详情和物品筛选值 |
| `stock.item.manage` | 物品管理 | 创建、修改、删除物品 |
| `stock.location.read` | 库位管理 | 查看库位分组树和库位列表 |
| `stock.location.manage` | 库位管理 | 管理库位分组、库位和整批次移库 |
| `stock.inbound.create` | 入库 | 创建入库单 |
| `stock.inbound.read` | 入库 | 查看入库单列表、详情和入库历史筛选值 |
| `stock.inbound.approve` | 入库 | 审批或拒绝入库单；与创建权限同时具备时可直接入库 |
| `stock.outbound.create` | 出库 | 创建出库单 |
| `stock.outbound.read` | 出库 | 查看出库单列表、详情和出库历史筛选值 |
| `stock.outbound.approve` | 出库 | 审批或拒绝出库单 |
| `stock.template.read` | 分类与模板 | 查看物品分类和物品属性模板 |
| `stock.template.manage` | 分类与模板 | 管理物品分类和物品属性模板 |
| `stock.dashboard.read` | 总览看板 | 查看看板总览和趋势 |
| `stock.substitute.read` | 替代料管理 | 查看替代关系 |
| `stock.substitute.manage` | 替代料管理 | 整体替换或删除替代关系 |
| `audit.read` | 事件日志 | 查看审计日志 |

权限分配由用户管理接口直接写入用户权限关系。
首个用户获得全部内置权限；后续用户默认无权限，需要由拥有 `user.permissions.update` 的用户显式分配。
