// 本文件拥有 frontend 业务 API 共用的分页响应契约；它不决定具体页面的分页交互。

/** 通用分页响应。 */
export interface PaginatedResponse<TItem> {
  /** 当前页数据。 */
  items: TItem[]
  /** 满足条件的总记录数。 */
  total: number
  /** 当前页码，从 1 开始。 */
  page: number
  /** 当前每页数量。 */
  page_size: number
  /** 总页数；无数据时为 0。 */
  total_pages: number
}
