<!-- 本组件拥有无依赖的 HSV、HEX 和快捷色板选择交互；它不生成图片或保存业务字段。 -->
<template>
  <div class="attribute-color-picker">
    <div
      ref="saturationPlane"
      class="attribute-color-picker__saturation"
      :style="{ backgroundColor: `hsl(${hue} 100% 50%)` }"
      role="slider"
      tabindex="0"
      aria-label="颜色饱和度和亮度"
      aria-valuemin="0"
      aria-valuemax="100"
      :aria-valuenow="Math.round(saturation * 100)"
      :aria-valuetext="`饱和度 ${Math.round(saturation * 100)}%，亮度 ${Math.round(value * 100)}%`"
      @pointerdown="startSaturationDrag"
      @pointermove="moveSaturationDrag"
      @pointerup="finishSaturationDrag"
      @pointercancel="cancelDrag"
      @keydown="handleSaturationKeydown"
    >
      <span
        class="attribute-color-picker__saturation-handle"
        :style="{ left: `${saturation * 100}%`, top: `${(1 - value) * 100}%`, backgroundColor: selectedColor }"
      />
    </div>

    <div
      ref="hueTrack"
      class="attribute-color-picker__hue"
      role="slider"
      tabindex="0"
      aria-label="色相"
      aria-valuemin="0"
      aria-valuemax="360"
      :aria-valuenow="Math.round(hue)"
      @pointerdown="startHueDrag"
      @pointermove="moveHueDrag"
      @pointerup="finishHueDrag"
      @pointercancel="cancelDrag"
      @keydown="handleHueKeydown"
    >
      <span class="attribute-color-picker__hue-handle" :style="{ left: `${(hue / 360) * 100}%` }" />
    </div>

    <div class="attribute-color-picker__value-row">
      <span class="attribute-color-picker__current" :style="{ backgroundColor: selectedColor }" aria-hidden="true" />
      <label class="attribute-color-picker__hex">
        <span>HEX</span>
        <span class="attribute-color-picker__hex-control">
          <span>#</span>
          <input
            v-model="hexText"
            :name="hexInputName"
            type="text"
            maxlength="6"
            inputmode="text"
            autocomplete="off"
            spellcheck="false"
            aria-label="十六进制颜色"
            @input="applyHexDraft(false)"
            @blur="applyHexDraft(true)"
            @keydown.enter.prevent="applyHexDraft(true)"
          />
        </span>
      </label>
    </div>

    <div class="attribute-color-picker__swatches" aria-label="常用颜色">
      <button
        v-for="color in palette"
        :key="color"
        type="button"
        :style="{ backgroundColor: color }"
        :title="color"
        :aria-label="`选择颜色 ${color}`"
        :aria-pressed="selectedColor === color"
        @click="selectSwatch(color)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, useId, watch } from 'vue'

const props = defineProps<{ modelValue: string }>()
const emit = defineEmits<{
  'update:modelValue': [value: string]
  commit: [value: string]
}>()

const palette = [
  '#9d2832', '#d4664f', '#d59b45', '#c9a978', '#86a45d',
  '#4f91aa', '#3f6f9f', '#657fba', '#8174b2', '#ad6f92',
]
const hexInputName = `color_hex_${useId()}`
const initial = hexToHsv(props.modelValue)
const hue = ref(initial.h)
const saturation = ref(initial.s)
const value = ref(initial.v)
const hexText = ref(normalizeHex(props.modelValue).slice(1))
const saturationPlane = ref<HTMLElement | null>(null)
const hueTrack = ref<HTMLElement | null>(null)
const activeDrag = ref<'saturation' | 'hue' | null>(null)
const selectedColor = computed(() => hsvToHex(hue.value, saturation.value, value.value))

watch(() => props.modelValue, (next) => {
  const normalized = normalizeHex(next)
  if (normalized === selectedColor.value) return
  const hsv = hexToHsv(normalized)
  hue.value = hsv.h
  saturation.value = hsv.s
  value.value = hsv.v
  hexText.value = normalized.slice(1)
})

watch(selectedColor, (next) => {
  hexText.value = next.slice(1)
  emit('update:modelValue', next)
})

function startSaturationDrag(event: PointerEvent): void {
  activeDrag.value = 'saturation'
  saturationPlane.value?.setPointerCapture(event.pointerId)
  updateSaturation(event)
}

function moveSaturationDrag(event: PointerEvent): void {
  if (activeDrag.value === 'saturation') updateSaturation(event)
}

function finishSaturationDrag(event: PointerEvent): void {
  if (activeDrag.value !== 'saturation') return
  updateSaturation(event)
  saturationPlane.value?.releasePointerCapture(event.pointerId)
  activeDrag.value = null
  emit('commit', selectedColor.value)
}

function startHueDrag(event: PointerEvent): void {
  activeDrag.value = 'hue'
  hueTrack.value?.setPointerCapture(event.pointerId)
  updateHue(event)
}

function moveHueDrag(event: PointerEvent): void {
  if (activeDrag.value === 'hue') updateHue(event)
}

function finishHueDrag(event: PointerEvent): void {
  if (activeDrag.value !== 'hue') return
  updateHue(event)
  hueTrack.value?.releasePointerCapture(event.pointerId)
  activeDrag.value = null
  emit('commit', selectedColor.value)
}

function cancelDrag(): void {
  activeDrag.value = null
}

function updateSaturation(event: PointerEvent): void {
  const rect = saturationPlane.value?.getBoundingClientRect()
  if (!rect) return
  saturation.value = clamp((event.clientX - rect.left) / rect.width)
  value.value = 1 - clamp((event.clientY - rect.top) / rect.height)
}

function updateHue(event: PointerEvent): void {
  const rect = hueTrack.value?.getBoundingClientRect()
  if (!rect) return
  hue.value = clamp((event.clientX - rect.left) / rect.width) * 360
}

function handleSaturationKeydown(event: KeyboardEvent): void {
  const step = event.shiftKey ? 0.1 : 0.02
  if (event.key === 'ArrowLeft') saturation.value = clamp(saturation.value - step)
  else if (event.key === 'ArrowRight') saturation.value = clamp(saturation.value + step)
  else if (event.key === 'ArrowDown') value.value = clamp(value.value - step)
  else if (event.key === 'ArrowUp') value.value = clamp(value.value + step)
  else if (event.key === 'Enter' || event.key === ' ') emit('commit', selectedColor.value)
  else return
  event.preventDefault()
}

function handleHueKeydown(event: KeyboardEvent): void {
  const step = event.shiftKey ? 15 : 3
  if (event.key === 'ArrowLeft' || event.key === 'ArrowDown') hue.value = (hue.value - step + 360) % 360
  else if (event.key === 'ArrowRight' || event.key === 'ArrowUp') hue.value = (hue.value + step) % 360
  else if (event.key === 'Enter' || event.key === ' ') emit('commit', selectedColor.value)
  else return
  event.preventDefault()
}

function applyHexDraft(commit: boolean): void {
  const candidate = `#${hexText.value.trim()}`
  if (!/^#[0-9a-f]{6}$/i.test(candidate)) {
    if (commit) hexText.value = selectedColor.value.slice(1)
    return
  }
  const normalized = normalizeHex(candidate)
  const hsv = hexToHsv(normalized)
  hue.value = hsv.h
  saturation.value = hsv.s
  value.value = hsv.v
  if (commit) emit('commit', normalized)
}

function selectSwatch(color: string): void {
  const hsv = hexToHsv(color)
  hue.value = hsv.h
  saturation.value = hsv.s
  value.value = hsv.v
  emit('commit', normalizeHex(color))
}

function clamp(number: number): number {
  return Math.min(1, Math.max(0, number))
}

function normalizeHex(color: string): string {
  const normalized = color.trim().toLowerCase()
  return /^#[0-9a-f]{6}$/.test(normalized) ? normalized : '#6f2a36'
}

function hexToHsv(color: string): { h: number; s: number; v: number } {
  const hex = normalizeHex(color).slice(1)
  const red = Number.parseInt(hex.slice(0, 2), 16) / 255
  const green = Number.parseInt(hex.slice(2, 4), 16) / 255
  const blue = Number.parseInt(hex.slice(4, 6), 16) / 255
  const max = Math.max(red, green, blue)
  const min = Math.min(red, green, blue)
  const delta = max - min
  let nextHue = 0
  if (delta > 0) {
    if (max === red) nextHue = 60 * (((green - blue) / delta) % 6)
    else if (max === green) nextHue = 60 * ((blue - red) / delta + 2)
    else nextHue = 60 * ((red - green) / delta + 4)
  }
  if (nextHue < 0) nextHue += 360
  return { h: nextHue, s: max === 0 ? 0 : delta / max, v: max }
}

function hsvToHex(nextHue: number, nextSaturation: number, nextValue: number): string {
  const chroma = nextValue * nextSaturation
  const segment = nextHue / 60
  const intermediate = chroma * (1 - Math.abs((segment % 2) - 1))
  let red = 0
  let green = 0
  let blue = 0
  if (segment < 1) [red, green] = [chroma, intermediate]
  else if (segment < 2) [red, green] = [intermediate, chroma]
  else if (segment < 3) [green, blue] = [chroma, intermediate]
  else if (segment < 4) [green, blue] = [intermediate, chroma]
  else if (segment < 5) [red, blue] = [intermediate, chroma]
  else [red, blue] = [chroma, intermediate]
  const match = nextValue - chroma
  return `#${[red, green, blue].map((channel) => Math.round((channel + match) * 255).toString(16).padStart(2, '0')).join('')}`
}
</script>

<style scoped>
.attribute-color-picker {
  display: grid;
  gap: 10px;
  padding: 8px;
  border-top: 1px solid var(--color-border);
}

.attribute-color-picker__saturation {
  position: relative;
  height: 132px;
  overflow: hidden;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-sm);
  background-image:
    linear-gradient(to top, #000, transparent),
    linear-gradient(to right, #fff, transparent);
  cursor: crosshair;
  touch-action: none;
}

.attribute-color-picker__saturation:focus-visible,
.attribute-color-picker__hue:focus-visible {
  outline: 2px solid var(--color-accent);
  outline-offset: 2px;
}

.attribute-color-picker__saturation-handle {
  position: absolute;
  width: 14px;
  height: 14px;
  border: 2px solid #fff;
  border-radius: 50%;
  box-shadow: 0 0 0 1px rgb(23 32 42 / 55%);
  pointer-events: none;
  transform: translate(-50%, -50%);
}

.attribute-color-picker__hue {
  position: relative;
  height: 14px;
  border-radius: 999px;
  background: linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00);
  cursor: ew-resize;
  touch-action: none;
}

.attribute-color-picker__hue-handle {
  position: absolute;
  top: 50%;
  width: 8px;
  height: 20px;
  border: 2px solid #fff;
  border-radius: 3px;
  background: transparent;
  box-shadow: 0 0 0 1px rgb(23 32 42 / 45%);
  pointer-events: none;
  transform: translate(-50%, -50%);
}

.attribute-color-picker__value-row {
  display: grid;
  grid-template-columns: 36px minmax(0, 1fr);
  align-items: end;
  gap: 10px;
}

.attribute-color-picker__current {
  width: 36px;
  height: 36px;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-sm);
}

.attribute-color-picker__hex {
  display: grid;
  gap: 4px;
}

.attribute-color-picker__hex > span:first-child {
  color: var(--color-muted);
  font-size: 10px;
  font-weight: 650;
}

.attribute-color-picker__hex-control {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  gap: 2px;
  min-height: 36px;
  padding: 0 8px;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-sm);
  background: var(--color-surface);
  color: var(--color-muted);
  font-family: var(--font-mono);
  font-size: 12px;
}

.attribute-color-picker__hex-control:focus-within {
  border-color: var(--color-accent);
}

.attribute-color-picker__hex-control input {
  min-width: 0;
  padding: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--color-text);
  font-family: inherit;
  text-transform: uppercase;
}

.attribute-color-picker__swatches {
  display: grid;
  grid-template-columns: repeat(10, minmax(0, 1fr));
  gap: 5px;
}

.attribute-color-picker__swatches button {
  aspect-ratio: 1;
  min-width: 0;
  padding: 0;
  border: 1px solid rgb(23 32 42 / 18%);
  border-radius: 3px;
}

.attribute-color-picker__swatches button[aria-pressed='true'] {
  outline: 2px solid var(--color-text);
  outline-offset: 1px;
}
</style>
