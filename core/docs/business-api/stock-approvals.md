# 库存审批 API

库存审批接口承载会改变入库/出库单据状态并写入库存流水或审计事件的审批动作。入库/出库列表与详情仍归各自单据接口。

### `POST /api/stock-approvals/inbound/{id}/approve`

审批入库单。服务端只允许审批 `pending` 单据；审批前按物品关联模板校验 `ext_attributes`，审批事务内再次确认明细库位仍未删除，然后生成批次、写入库存流水和审计事件。

- 权限：`stock.inbound.approve`
- 响应：`200` + `InboundResponse`，状态为 `approved`
- 错误：`400` 扩展属性不满足模板约束 / `404` 入库单或库位不存在 / `409` 单据不是 `pending`

### `POST /api/stock-approvals/inbound/{id}/reject`

拒绝入库单。拒绝只更新单据状态并写审计事件，不改变库存；被拒绝单据不能再审批。

- 权限：`stock.inbound.approve`
- 响应：`200` + `InboundResponse`，状态为 `rejected`
- 错误：`404` 入库单不存在 / `409` 单据不是 `pending`

### `POST /api/stock-approvals/outbound/{id}/approve`

审批出库单。服务端只允许审批 `pending` 单据；明细指定 `batch_id` 时只扣指定批次，未指定时按 `expires_at ASC NULLS LAST, received_at ASC, id ASC` 的 FIFO 规则扣减。库存不足或指定批次不可用时返回冲突并回滚整个审批事务。

- 权限：`stock.outbound.approve`
- 响应：`200` + `OutboundResponse`，状态为 `approved`
- 错误：`404` 出库单不存在 / `409` 单据不是 `pending` 或库存不足

### `POST /api/stock-approvals/outbound/{id}/reject`

拒绝出库单。拒绝只更新单据状态并写审计事件，不扣减库存；被拒绝单据不能再审批。

- 权限：`stock.outbound.approve`
- 响应：`200` + `OutboundResponse`，状态为 `rejected`
- 错误：`404` 出库单不存在 / `409` 单据不是 `pending`
