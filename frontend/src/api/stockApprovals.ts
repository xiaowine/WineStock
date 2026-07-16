// 本文件拥有库存审批写操作的 HTTP 契约；它不查询队列、不管理审核会话或预测库存结果。
import { apiClient } from './client'
import type { InboundOrderResponse } from './inboundOrders'
import type { OutboundOrderResponse } from './outboundOrders'

/** 审批 pending 入库单；服务端在事务内生成批次、增加库存并写审计事件。 */
export function approveInboundOrder(id: number) {
  return apiClient.request<InboundOrderResponse>(`/api/stock-approvals/inbound/${id}/approve`, {
    method: 'POST',
  })
}

/** 拒绝 pending 入库单；服务端只更新单据状态，不增加库存。 */
export function rejectInboundOrder(id: number) {
  return apiClient.request<InboundOrderResponse>(`/api/stock-approvals/inbound/${id}/reject`, {
    method: 'POST',
  })
}

/** 审批 pending 出库单；服务端按指定批次或 FIFO 扣减库存。 */
export function approveOutboundOrder(id: number) {
  return apiClient.request<OutboundOrderResponse>(`/api/stock-approvals/outbound/${id}/approve`, {
    method: 'POST',
  })
}

/** 拒绝 pending 出库单；服务端只更新单据状态，不扣减库存。 */
export function rejectOutboundOrder(id: number) {
  return apiClient.request<OutboundOrderResponse>(`/api/stock-approvals/outbound/${id}/reject`, {
    method: 'POST',
  })
}
