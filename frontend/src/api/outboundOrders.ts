import { apiClient } from './client'
import type { PaginatedResponse } from './pagination'

export type OutboundOrderStatus = 'pending' | 'approved' | 'rejected'
export interface OutboundOrderItemResponse { id:number; order_id:number; item_id:number; item_name:string; item_sku:string; item_unit:string; item_image_file_id:number; quantity:number; batch_id:number|null; location_id:number|null; location_name:string|null; created_at:string }
export interface OutboundOrderResponse { id:number; destination:string; status:OutboundOrderStatus; notes:string|null; created_by_user_id:number|null; approved_by_user_id:number|null; rejected_by_user_id:number|null; created_at:string; updated_at:string; approved_at:string|null; rejected_at:string|null; items:OutboundOrderItemResponse[] }
export interface OutboundOrderListQuery { page:number; page_size:number; search?:string; status?:OutboundOrderStatus; date_from?:string; date_to?:string }
export const listOutboundOrders = (query:OutboundOrderListQuery, signal?:AbortSignal) => apiClient.request<PaginatedResponse<OutboundOrderResponse>>('/api/outbound',{query:{...query},signal})
export const getOutboundOrder = (id:number, signal?:AbortSignal) => apiClient.request<OutboundOrderResponse>(`/api/outbound/${id}`,{signal})
