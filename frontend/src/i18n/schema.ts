// i18n 消息键结构基准类型：以 zh-CN 语言包为准，供 vue-i18n 类型增强使用。
import type { zhCN } from "./locales/zh-CN";

/** 全部消息键的类型结构；en-US 在聚合器内经 satisfies 强制与 zh-CN 对齐。 */
export type MessageSchema = typeof zhCN;

/** 递归展开嵌套消息对象的叶子路径（点分隔），例如 "error.networkUnavailable"。 */
type LeafPaths<T, Prefix extends string = ""> = {
  [K in keyof T & string]: T[K] extends string
    ? `${Prefix}${K}`
    : LeafPaths<T[K], `${Prefix}${K}.`>;
}[keyof T & string];

/** 消息键的点路径联合；动态键（如 error.<code>）翻译时经此类型收窄。 */
export type MessageKeyPath = LeafPaths<MessageSchema>;

