// 本文件拥有生成契约（generated/schema.d.ts）的桥接辅助类型；不定义业务 DTO，也不发起请求。
// 各 api 模块通过这里把生成 schema 映射回稳定导出名；generated 目录只允许 pnpm gen:api-types 写入。
import type { components } from "./generated/schema";

/** 按组件名索引生成 schema 的快捷类型。 */
export type ApiSchema<K extends keyof components["schemas"]> = components["schemas"][K];

/**
 * 响应字段必填化：serde 序列化 Option 字段时 None 输出 null，响应字段始终存在；
 * utoipa 却把 Option 字段标记为可缺省。本映射递归移除可缺省标记，不改变字段值类型。
 * 只用于响应类型；请求类型保持生成的可缺省语义（serde 反序列化允许省略 Option 字段）。
 */
export type ApiResponse<T> = T extends (infer U)[]
  ? ApiResponse<U>[]
  : T extends object
    ? { [K in keyof T]-?: ApiResponse<T[K]> }
    : T;
