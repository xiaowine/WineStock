// 本文件拥有两类属性模板共享的字段 DTO，属于 frontend HTTP 类型边界；它不发起请求。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定。
import type { ApiResponse, ApiSchema } from "./contract";

export type TemplateFieldType = ApiSchema<"TemplateFieldType">;

export type TemplateFieldResponse = ApiResponse<ApiSchema<"TemplateFieldResponse">>;

/** 创建或整体保存模板时的字段定义；生成 schema 名为 TemplateFieldDef。 */
export type TemplateFieldRequest = ApiSchema<"TemplateFieldDef">;
