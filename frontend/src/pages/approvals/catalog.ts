// 本文件定义入库与出库审批页共享的领域目录；它只适配 HTTP DTO，不拥有页面状态或视觉实现。
import {
  getInboundOrder,
  listInboundOrders,
  type InboundOrderResponse,
} from "../../api/inboundOrders";
import {
  getOutboundOrder,
  listOutboundOrders,
  type OutboundOrderResponse,
} from "../../api/outboundOrders";
import type { PaginatedResponse } from "../../api/pagination";
import {
  approveInboundOrder,
  approveOutboundOrder,
  rejectInboundOrder,
  rejectOutboundOrder,
} from "../../api/stockApprovals";
import { translateMessageOrNull } from "../../i18n";

/** 库存审批领域。 */
export type ApprovalKind = "inbound" | "outbound";

/** 共享工作台持有的判别联合单据。 */
export type ApprovalRecord =
  | { kind: "inbound"; order: InboundOrderResponse }
  | { kind: "outbound"; order: OutboundOrderResponse };

/** 待审批队列的稳定查询参数。 */
export interface ApprovalListQuery {
  page: number;
  page_size: number;
  search?: string;
  date_from?: string;
  date_to?: string;
}

/** 单个审批领域注入共享工作台的业务配置。 */
export interface ApprovalCatalog {
  kind: ApprovalKind;
  pageSubtitle: string;
  searchPlaceholder: string;
  emptyLabel: string;
  noResultLabel: string;
  contextLabel: string;
  approveConsequence: string;
  rejectConsequence: string;
  list(query: ApprovalListQuery, signal?: AbortSignal): Promise<PaginatedResponse<ApprovalRecord>>;
  get(id: number, signal?: AbortSignal): Promise<ApprovalRecord>;
  approve(id: number): Promise<ApprovalRecord>;
  reject(id: number): Promise<ApprovalRecord>;
}

const catalogs: Record<ApprovalKind, ApprovalCatalog> = {
  inbound: {
    kind: "inbound",
    pageSubtitle: "approvals.inboundSubtitle",
    searchPlaceholder: "approvals.inboundSearchPlaceholder",
    emptyLabel: "approvals.inboundEmptyLabel",
    noResultLabel: "approvals.inboundNoResultLabel",
    contextLabel: "approvals.contextSource",
    approveConsequence: "approvals.inboundApproveConsequence",
    rejectConsequence: "approvals.inboundRejectConsequence",
    async list(query, signal) {
      const response = await listInboundOrders({ ...query, status: "pending" }, signal);
      return {
        ...response,
        items: response.items.map((order) => ({
          kind: "inbound" as const,
          order,
        })),
      };
    },
    async get(id, signal) {
      return { kind: "inbound", order: await getInboundOrder(id, signal) };
    },
    async approve(id) {
      return { kind: "inbound", order: await approveInboundOrder(id) };
    },
    async reject(id) {
      return { kind: "inbound", order: await rejectInboundOrder(id) };
    },
  },
  outbound: {
    kind: "outbound",
    pageSubtitle: "approvals.outboundSubtitle",
    searchPlaceholder: "approvals.outboundSearchPlaceholder",
    emptyLabel: "approvals.outboundEmptyLabel",
    noResultLabel: "approvals.outboundNoResultLabel",
    contextLabel: "approvals.contextDestination",
    approveConsequence: "approvals.outboundApproveConsequence",
    rejectConsequence: "approvals.outboundRejectConsequence",
    async list(query, signal) {
      const response = await listOutboundOrders({ ...query, status: "pending" }, signal);
      return {
        ...response,
        items: response.items.map((order) => ({
          kind: "outbound" as const,
          order,
        })),
      };
    },
    async get(id, signal) {
      return { kind: "outbound", order: await getOutboundOrder(id, signal) };
    },
    async approve(id) {
      return { kind: "outbound", order: await approveOutboundOrder(id) };
    },
    async reject(id) {
      return { kind: "outbound", order: await rejectOutboundOrder(id) };
    },
  },
};

/** 返回路由页对应的审批领域配置。 */
export function getApprovalCatalog(kind: ApprovalKind): ApprovalCatalog {
  return catalogs[kind];
}

/** 返回单据 ID。 */
export function approvalId(record: ApprovalRecord): number {
  return record.order.id;
}

/** 返回单据来源或去向。 */
export function approvalContext(record: ApprovalRecord): string {
  return (
    record.kind === "inbound"
      ? record.order.source || translateMessageOrNull("approvals.unrecordedSource")
      : record.order.destination || translateMessageOrNull("approvals.unrecordedDestination")
  ) ?? "";
}
