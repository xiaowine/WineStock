// 本文件拥有 frontend i18n 的纯类型、持久化键与语言决议逻辑；它不访问 DOM、存储或 vue-i18n 实例。
// 语言偏好属于本机偏好（仿 theme/consent），不进 Shell 运行配置。

/** 受支持的应用语言；zh-CN 为产品默认语言与兜底语言。 */
export const SUPPORTED_LOCALES = ["zh-CN", "en-US"] as const;

/** 应用语言联合类型。 */
export type AppLocale = (typeof SUPPORTED_LOCALES)[number];

/** 语言偏好的版本化 localStorage 键。 */
export const LOCALE_STORAGE_KEY = "winestock.i18n.locale.v1";

/** 校验来自 localStorage 或跨标签页事件的未知值，非法值返回 null。 */
export function parseLocale(value: unknown): AppLocale | null {
  return typeof value === "string" && (SUPPORTED_LOCALES as readonly string[]).includes(value)
    ? (value as AppLocale)
    : null;
}

/** 按浏览器语言解析初始语言：zh* → zh-CN，其余 → en-US。 */
export function resolveLocaleFromNavigator(language: string): AppLocale {
  return language.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US";
}
