// 本文件拥有主题偏好的纯类型、持久化值校验和最终主题解析；不访问 DOM、存储或平台接口。

export const THEME_STORAGE_KEY = "winestock.theme.preference.v1";

export const THEME_PREFERENCES = ["system", "light", "dark"] as const;

/** 当前设备可选择的主题偏好；system 表示实时跟随操作系统。 */
export type ThemePreference = (typeof THEME_PREFERENCES)[number];

/** 页面、浏览器外观和平台系统栏最终使用的二态主题。 */
export type ResolvedTheme = Exclude<ThemePreference, "system">;

/** 校验来自 localStorage 或跨标签页事件的未知值，非法值统一回退到跟随系统。 */
export function parseThemePreference(value: unknown): ThemePreference {
  return typeof value === "string" && THEME_PREFERENCES.includes(value as ThemePreference)
    ? (value as ThemePreference)
    : "system";
}

/** 把三态用户偏好与当前系统媒体查询合并为实际使用的浅色或深色主题。 */
export function resolveTheme(preference: ThemePreference, systemDark: boolean): ResolvedTheme {
  return preference === "system" ? (systemDark ? "dark" : "light") : preference;
}
