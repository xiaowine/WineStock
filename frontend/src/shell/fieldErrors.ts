// 运行配置字段错误的本地化辅助：把 {code, message} 契约元素翻译为表单展示文案。
// 本模块独立于 shell/contract.ts，保持契约文件零运行时依赖（node 测试内联加载契约）。
import { translateMessageOrNull } from "../i18n";
import type { RuntimeConfigField, RuntimeConfigFieldError } from "./contract";

/** 单条字段错误按 `bridge.<code>` 或 `validation.<code>` 本地化；未命中回退 message。 */
export function localizeFieldErrorMessage(entry: RuntimeConfigFieldError): string {
  return (
    translateMessageOrNull(`bridge.${entry.code}`) ??
    translateMessageOrNull(`validation.${entry.code}`) ??
    entry.message
  );
}

/** 把 {code, message} 字段错误聚合为本地化文案数组；未命中语言包时回退 message。 */
export function localizeRuntimeFieldErrors(
  fieldErrors: Partial<Record<RuntimeConfigField, readonly RuntimeConfigFieldError[]>>,
): Partial<Record<RuntimeConfigField, readonly string[]>> {
  const result: Partial<Record<RuntimeConfigField, string[]>> = {};
  for (const [field, errors] of Object.entries(fieldErrors) as [
    RuntimeConfigField,
    readonly RuntimeConfigFieldError[],
  ][]) {
    result[field] = errors.map((entry) => localizeFieldErrorMessage(entry));
  }
  return result;
}
