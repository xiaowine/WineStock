// 本文件统一注册表单字段、清理字段错误并定位首个错误；具体业务校验规则仍由所属页面或模型负责。
import { inject, nextTick, provide, watch, type InjectionKey, type Ref } from 'vue'

type FormErrors = Readonly<Record<string, unknown>>
type FieldResolver = () => HTMLElement | null

interface FormValidationContext {
  registerField: (key: string, resolver: FieldResolver) => () => void
  clearFieldError: (key: string) => void
}

const formValidationKey: InjectionKey<FormValidationContext> = Symbol('form-validation')

/** 为当前组件子树提供字段注册和首错定位能力。 */
export function useFormValidation<T extends FormErrors>(errors: Ref<T>) {
  const fields = new Map<string, Set<FieldResolver>>()
  let suppressNextFocus = false

  function registerField(key: string, resolver: FieldResolver): () => void {
    const resolvers = fields.get(key) ?? new Set<FieldResolver>()
    resolvers.add(resolver)
    fields.set(key, resolvers)
    return () => {
      resolvers.delete(resolver)
      if (resolvers.size === 0) fields.delete(key)
    }
  }

  function clearFieldError(key: string): void {
    if (!key || !(key in errors.value)) return
    const next: Record<string, unknown> = { ...errors.value }
    delete next[key]
    suppressNextFocus = true
    errors.value = next as T
  }

  function clearErrors(): void {
    errors.value = {} as T
  }

  async function focusFirstError(keys = Object.keys(errors.value)): Promise<void> {
    await nextTick()
    for (const key of keys) {
      const resolvers = fields.get(key)
      if (!resolvers) continue
      for (const resolve of resolvers) {
        const container = resolve()
        if (!container) continue
        const target = container.matches('input, textarea, select, button, [tabindex]:not([tabindex="-1"])')
          ? container
          : container.querySelector<HTMLElement>('input, textarea, select, button, [tabindex]:not([tabindex="-1"])')
        container.scrollIntoView({ behavior: 'smooth', block: 'center' })
        target?.focus({ preventScroll: true })
        return
      }
    }
  }

  const context: FormValidationContext = { registerField, clearFieldError }
  provide(formValidationKey, context)
  watch(errors, (next) => {
    if (suppressNextFocus) {
      suppressNextFocus = false
      return
    }
    const keys = Object.keys(next)
    if (keys.length > 0) void focusFirstError(keys)
  }, { flush: 'post' })

  return { clearErrors, clearFieldError, focusFirstError }
}

export function useFormValidationContext(): FormValidationContext | null {
  return inject(formValidationKey, null)
}
