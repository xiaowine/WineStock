<!-- 本组件拥有项目通用选择控件、浮层列表和键盘交互；它不拥有字段标题、校验规则或业务选项。 -->
<template>
  <span
    ref="root"
    class="select-control"
    :class="[attrs.class, { 'select-control--compact': compact, 'select-control--open': open }]"
    :style="attrs.style"
  >
    <button
      ref="trigger"
      v-bind="triggerAttrs"
      class="select-control__trigger"
      type="button"
      role="combobox"
      :disabled="disabled"
      :aria-expanded="open"
      :aria-controls="listboxId"
      :aria-haspopup="'listbox'"
      @click="toggle"
      @keydown="handleTriggerKeydown"
    >
      <span :class="{ 'select-control__value--placeholder': selectedOption?.placeholder }">
        {{ selectedOption?.label ?? '请选择' }}
      </span>
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="m4 6 4 4 4-4" />
      </svg>
    </button>
    <input v-if="name" type="hidden" :name="name" :value="serializedValue" />

    <Teleport to="body">
      <Transition name="select-options">
        <div
          v-if="open"
          :id="listboxId"
          ref="listbox"
          class="select-control__options"
          role="listbox"
          :aria-label="typeof attrs['aria-label'] === 'string' ? attrs['aria-label'] : undefined"
          :style="popoverStyle"
          @keydown="handleListboxKeydown"
        >
          <template v-for="(option, index) in options" :key="option.key">
            <div v-if="option.group && option.group !== options[index - 1]?.group" class="select-control__group">
              {{ option.group }}
            </div>
            <button
              :ref="(element) => setOptionElement(element, index)"
              class="select-control__option"
              :class="{ 'select-control__option--selected': sameValue(option.value, model) }"
              type="button"
              role="option"
              :aria-selected="sameValue(option.value, model)"
              :disabled="option.disabled"
              @click="choose(option)"
            >
              <span>{{ option.label }}</span>
            </button>
          </template>
        </div>
      </Transition>
    </Teleport>
  </span>
</template>

<script setup lang="ts">
import {
  Comment, Fragment, Text, computed, nextTick, onBeforeUnmount, ref, useAttrs, useId, useSlots,
  type ComponentPublicInstance, type CSSProperties, type VNode,
} from 'vue'

interface NormalizedOption {
  key: string
  label: string
  value: unknown
  disabled: boolean
  placeholder: boolean
  group?: string
}

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<{
  compact?: boolean
  disabled?: boolean
  name?: string
}>(), {
  compact: false,
  disabled: false,
  name: '',
})

const emit = defineEmits<{
  change: [value: unknown]
}>()

const model = defineModel<unknown>()
const attrs = useAttrs()
const slots = useSlots()
const uid = useId()
const listboxId = `select-options-${uid}`
const root = ref<HTMLElement | null>(null)
const trigger = ref<HTMLButtonElement | null>(null)
const listbox = ref<HTMLElement | null>(null)
const open = ref(false)
const activeIndex = ref(-1)
const optionElements = new Map<number, HTMLButtonElement>()
const popoverStyle = ref<CSSProperties>({})

const triggerAttrs = computed(() => Object.fromEntries(
  Object.entries(attrs).filter(([key]) => !['class', 'style', 'name', 'disabled', 'required'].includes(key)),
))
const options = computed(() => normalizeNodes(slots.default?.() ?? []))
const selectedOption = computed(() => options.value.find((option) => sameValue(option.value, model.value)))
const serializedValue = computed(() => model.value === null || model.value === undefined ? '' : String(model.value))

function normalizeNodes(nodes: VNode[], group?: string): NormalizedOption[] {
  const normalized: NormalizedOption[] = []
  for (const node of nodes) {
    if (node.type === Comment || node.type === Text) continue
    if (node.type === Fragment) {
      normalized.push(...normalizeNodes(asNodes(node.children), group))
      continue
    }
    if (node.type === 'optgroup') {
      normalized.push(...normalizeNodes(asNodes(node.children), String(node.props?.label ?? '')))
      continue
    }
    if (node.type === 'option') {
      const label = nodeText(node)
      const hasValue = Object.prototype.hasOwnProperty.call(node.props ?? {}, 'value')
      const value = hasValue ? node.props?.value : label
      normalized.push({
        key: `${group ?? ''}:${String(value ?? '')}:${normalized.length}`,
        label,
        value,
        disabled: Boolean(node.props?.disabled),
        placeholder: hasValue && (value === '' || value === null || value === undefined),
        group,
      })
      continue
    }
    normalized.push(...normalizeNodes(asNodes(node.children), group))
  }
  return normalized
}

function asNodes(children: VNode['children']): VNode[] {
  return Array.isArray(children) ? children as VNode[] : []
}

function nodeText(node: VNode): string {
  if (typeof node.children === 'string') return node.children.trim()
  return asNodes(node.children).map((child) => typeof child.children === 'string' ? child.children : nodeText(child)).join('').trim()
}

function sameValue(left: unknown, right: unknown): boolean {
  if (Object.is(left, right)) return true
  if (left === null || left === undefined || right === null || right === undefined) return false
  return String(left) === String(right)
}

function toggle(): void {
  if (props.disabled) return
  open.value ? close() : void show()
}

async function show(): Promise<void> {
  window.dispatchEvent(new CustomEvent('winestock-select-open', { detail: uid }))
  open.value = true
  activeIndex.value = Math.max(0, options.value.findIndex((option) => sameValue(option.value, model.value)))
  window.addEventListener('pointerdown', handleOutsidePointer)
  window.addEventListener('resize', positionPopover)
  window.addEventListener('scroll', positionPopover, true)
  window.addEventListener('winestock-select-open', handleOtherSelect as EventListener)
  await nextTick()
  positionPopover()
  focusActiveOption()
}

function close(restoreFocus = false): void {
  if (!open.value) return
  open.value = false
  removeWindowListeners()
  if (restoreFocus) void nextTick(() => trigger.value?.focus())
}

function choose(option: NormalizedOption): void {
  if (option.disabled) return
  model.value = option.value
  emit('change', option.value)
  close(true)
}

function handleTriggerKeydown(event: KeyboardEvent): void {
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp' || event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    void show()
  }
}

function handleListboxKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape' || event.key === 'Tab') {
    if (event.key === 'Escape') event.stopPropagation()
    close(event.key === 'Escape')
    return
  }
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    event.preventDefault()
    moveActive(event.key === 'ArrowDown' ? 1 : -1)
    return
  }
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    const option = options.value[activeIndex.value]
    if (option) choose(option)
  }
}

function moveActive(direction: number): void {
  if (!options.value.length) return
  let next = activeIndex.value
  do {
    next = (next + direction + options.value.length) % options.value.length
  } while (options.value[next]?.disabled && next !== activeIndex.value)
  activeIndex.value = next
  focusActiveOption()
}

function focusActiveOption(): void {
  void nextTick(() => optionElements.get(activeIndex.value)?.focus())
}

function setOptionElement(element: Element | ComponentPublicInstance | null, index: number): void {
  if (element instanceof HTMLButtonElement) optionElements.set(index, element)
  else optionElements.delete(index)
}

function positionPopover(): void {
  const triggerElement = trigger.value
  if (!triggerElement) return
  const anchor = triggerElement.getBoundingClientRect()
  const triggerStyle = window.getComputedStyle(triggerElement)
  const viewportHeight = window.visualViewport?.height ?? window.innerHeight
  const viewportWidth = window.visualViewport?.width ?? window.innerWidth
  const gap = 5
  const viewportPadding = 8
  const availableBelow = viewportHeight - anchor.bottom - gap - viewportPadding
  const availableAbove = anchor.top - gap - viewportPadding
  const estimatedHeight = Math.min(280, options.value.length * 38 + 12)
  const placeAbove = availableBelow < Math.min(180, estimatedHeight) && availableAbove > availableBelow
  const maxHeight = Math.max(120, Math.min(280, placeAbove ? availableAbove : availableBelow))
  const popoverWidth = Math.min(
    Math.max(anchor.width, preferredPopoverWidth(triggerStyle)),
    320,
    viewportWidth - viewportPadding * 2,
  )
  const popoverLeft = Math.min(
    Math.max(viewportPadding, anchor.left),
    Math.max(viewportPadding, viewportWidth - popoverWidth - viewportPadding),
  )
  popoverStyle.value = {
    left: `${popoverLeft}px`,
    top: placeAbove ? 'auto' : `${anchor.bottom + gap}px`,
    bottom: placeAbove ? `${viewportHeight - anchor.top + gap}px` : 'auto',
    width: `${popoverWidth}px`,
    maxWidth: `calc(100vw - ${viewportPadding * 2}px)`,
    maxHeight: `${maxHeight}px`,
    fontSize: triggerStyle.fontSize,
    lineHeight: triggerStyle.lineHeight,
  }
}

function preferredPopoverWidth(triggerStyle: CSSStyleDeclaration): number {
  const canvas = document.createElement('canvas')
  const context = canvas.getContext('2d')
  if (!context) return 0
  context.font = `${triggerStyle.fontWeight} ${triggerStyle.fontSize} ${triggerStyle.fontFamily}`
  const widestLabel = options.value.reduce((width, option) => Math.max(width, context.measureText(option.label).width), 0)
  // 包含选项与浮层的水平内边距、边框，并为字体测量误差保留少量余量。
  return Math.ceil(widestLabel + 30)
}

function handleOutsidePointer(event: PointerEvent): void {
  const target = event.target as Node
  if (!root.value?.contains(target) && !listbox.value?.contains(target)) close()
}

function handleOtherSelect(event: CustomEvent<string>): void {
  if (event.detail !== uid) close()
}

function removeWindowListeners(): void {
  window.removeEventListener('pointerdown', handleOutsidePointer)
  window.removeEventListener('resize', positionPopover)
  window.removeEventListener('scroll', positionPopover, true)
  window.removeEventListener('winestock-select-open', handleOtherSelect as EventListener)
}

onBeforeUnmount(removeWindowListeners)
</script>

<style scoped lang="scss">
.select-control {
  display: block;
  width: 100%;
  min-width: 0;
}

.select-control__trigger {
  display: grid;
  width: 100%;
  min-width: 0;
  min-height: 40px;
  grid-template-columns: minmax(0, 1fr) 14px;
  align-items: center;
  gap: 8px;
  padding: 0 10px;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-sm);
  outline: 0;
  background: var(--color-surface);
  color: var(--color-text);
  text-align: left;
}

.select-control__trigger > span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.select-control__value--placeholder {
  color: var(--color-muted);
}

.select-control__trigger svg {
  width: 14px;
  height: 14px;
  fill: none;
  stroke: currentcolor;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.select-control__trigger svg {
  color: var(--color-muted);
  transition: transform var(--motion-duration-fast) var(--motion-ease-standard);
}

.select-control--open .select-control__trigger svg {
  transform: rotate(180deg);
}

.select-control__trigger:focus-visible,
.select-control--open .select-control__trigger {
  border-color: var(--color-accent);
  box-shadow: 0 0 0 3px rgb(111 42 54 / 14%);
}

.select-control__trigger[aria-invalid='true'] {
  border-color: var(--color-danger);
}

.select-control__trigger:disabled {
  background: var(--color-surface-raised);
  color: var(--color-muted);
  cursor: not-allowed;
}

.select-control--compact .select-control__trigger {
  grid-template-columns: minmax(0, 1fr) 13px;
  gap: 6px;
  padding-right: 9px;
  padding-left: 8px;
}

.select-control__options {
  position: fixed;
  z-index: var(--z-dialog-popover);
  overflow-y: auto;
  padding: 5px;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  box-shadow: var(--shadow-menu);
}

.select-control__group {
  padding: 7px 8px 5px;
  color: var(--color-subtle);
  font-size: 11px;
  font-weight: 650;
}

.select-control__option {
  display: block;
  width: 100%;
  min-height: 36px;
  padding: 7px 8px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text);
  font-size: inherit;
  line-height: inherit;
  text-align: left;
}

.select-control__option > span {
  display: block;
  width: 100%;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.select-control__option--selected {
  background: var(--color-accent-soft);
  color: var(--color-accent);
  font-weight: 680;
}

.select-control__option:disabled {
  color: var(--color-subtle);
  cursor: not-allowed;
}

@media (hover: hover) and (pointer: fine) {
  .select-control__option:not(:disabled):hover {
    background: var(--color-surface-raised);
  }

  .select-control__option--selected:not(:disabled):hover {
    background: var(--color-accent-soft);
  }
}

.select-options-enter-active,
.select-options-leave-active {
  transition:
    opacity var(--motion-duration-fast) var(--motion-ease-standard),
    transform var(--motion-duration-fast) var(--motion-ease-standard);
}

.select-options-enter-from,
.select-options-leave-to {
  opacity: 0;
  transform: translateY(calc(0px - var(--motion-distance-small)));
}
</style>
