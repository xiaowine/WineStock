<!-- 本组件拥有项目通用日期时间选择交互；它不解释业务时间范围或转换 UTC。 -->
<template>
  <FormField :label="label" :control-id="triggerId" :error="error" :hint="hint" v-slot="{ describedBy, invalid }">
    <button
      :id="triggerId"
      class="date-time-field__trigger"
      type="button"
      :aria-expanded="open"
      :aria-invalid="invalid || undefined"
      :aria-describedby="describedBy"
      aria-haspopup="dialog"
      :disabled="disabled"
      @click="showPicker"
    >
      <span :class="{ 'date-time-field__placeholder': !modelValue }">{{ displayValue }}</span>
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="3" y="5" width="18" height="16" rx="2" />
        <path d="M16 3v4M8 3v4M3 10h18" />
        <path d="M8 14h.01M12 14h.01M16 14h.01M8 18h.01M12 18h.01" />
      </svg>
    </button>
  </FormField>

  <ModalDialog
    :open="open"
    :title="label"
    description="选择日期并设置精确到秒的时间。"
    compact
    nested
    @close="closePicker"
  >
    <div class="date-time-picker">
      <div class="date-time-picker__month">
        <button class="icon-button" type="button" title="上个月" aria-label="上个月" @click="changeMonth(-1)">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m15 18-6-6 6-6" /></svg>
        </button>
        <strong>{{ monthLabel }}</strong>
        <button class="icon-button" type="button" title="下个月" aria-label="下个月" @click="changeMonth(1)">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m9 18 6-6-6-6" /></svg>
        </button>
      </div>

      <div class="date-time-picker__weekdays" aria-hidden="true">
        <span v-for="weekday in weekdays" :key="weekday">{{ weekday }}</span>
      </div>
      <div class="date-time-picker__days" role="grid" :aria-label="monthLabel">
        <button
          v-for="calendarDay in calendarDays"
          :key="calendarDay.key"
          type="button"
          role="gridcell"
          :class="{
            'date-time-picker__day--outside': !calendarDay.currentMonth,
            'date-time-picker__day--today': calendarDay.today,
            'date-time-picker__day--selected': calendarDay.selected,
          }"
          :aria-label="calendarDay.ariaLabel"
          :aria-selected="calendarDay.selected"
          @click="selectDay(calendarDay)"
        >
          {{ calendarDay.day }}
        </button>
      </div>

      <fieldset class="date-time-picker__time">
        <legend>时间</legend>
        <label>
          <span>时</span>
          <input v-model="hour" inputmode="numeric" maxlength="2" aria-label="小时" @blur="hour = normalizePart(hour, 23)" />
        </label>
        <span aria-hidden="true">:</span>
        <label>
          <span>分</span>
          <input v-model="minute" inputmode="numeric" maxlength="2" aria-label="分钟" @blur="minute = normalizePart(minute, 59)" />
        </label>
        <span aria-hidden="true">:</span>
        <label>
          <span>秒</span>
          <input v-model="second" inputmode="numeric" maxlength="2" aria-label="秒" @blur="second = normalizePart(second, 59)" />
        </label>
      </fieldset>
    </div>

    <template #actions>
      <button class="text-button date-time-picker__clear" type="button" @click="clearValue">清除</button>
      <button class="secondary-button" type="button" @click="closePicker">取消</button>
      <button class="primary-button" type="button" @click="applyValue">应用</button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, ref, useId } from 'vue'
import ModalDialog from '../ModalDialog.vue'
import FormField from './FormField.vue'

interface CalendarDay {
  key: string
  year: number
  month: number
  day: number
  currentMonth: boolean
  today: boolean
  selected: boolean
  ariaLabel: string
}

const props = withDefaults(defineProps<{
  modelValue?: string
  label: string
  error?: string
  hint?: string
  disabled?: boolean
  placeholder?: string
}>(), {
  modelValue: '',
  error: '',
  hint: '',
  disabled: false,
  placeholder: '请选择日期和时间',
})

const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const triggerId = `date-time-${useId()}`
const weekdays = ['一', '二', '三', '四', '五', '六', '日']
const open = ref(false)
const year = ref(0)
const month = ref(0)
const day = ref(0)
const hour = ref('00')
const minute = ref('00')
const second = ref('00')

const displayValue = computed(() => props.modelValue ? formatDisplayValue(props.modelValue) : props.placeholder)
const monthLabel = computed(() => `${year.value}年${month.value + 1}月`)
const calendarDays = computed<CalendarDay[]>(() => {
  const firstWeekday = (new Date(year.value, month.value, 1).getDay() + 6) % 7
  const start = 1 - firstWeekday
  const today = new Date()
  return Array.from({ length: 42 }, (_, index) => {
    const date = new Date(year.value, month.value, start + index)
    return {
      key: `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`,
      year: date.getFullYear(),
      month: date.getMonth(),
      day: date.getDate(),
      currentMonth: date.getFullYear() === year.value && date.getMonth() === month.value,
      today: sameDate(date, today),
      selected: date.getFullYear() === year.value && date.getMonth() === month.value && date.getDate() === day.value,
      ariaLabel: `${date.getFullYear()}年${date.getMonth() + 1}月${date.getDate()}日`,
    }
  })
})

function showPicker(): void {
  if (props.disabled) return
  const value = parseLocalDateTime(props.modelValue) ?? new Date()
  year.value = value.getFullYear()
  month.value = value.getMonth()
  day.value = value.getDate()
  hour.value = pad(value.getHours())
  minute.value = pad(value.getMinutes())
  second.value = pad(value.getSeconds())
  open.value = true
}

function closePicker(): void {
  open.value = false
}

function changeMonth(offset: number): void {
  const next = new Date(year.value, month.value + offset, 1)
  year.value = next.getFullYear()
  month.value = next.getMonth()
  day.value = Math.min(day.value, new Date(year.value, month.value + 1, 0).getDate())
}

function selectDay(value: CalendarDay): void {
  year.value = value.year
  month.value = value.month
  day.value = value.day
}

function applyValue(): void {
  hour.value = normalizePart(hour.value, 23)
  minute.value = normalizePart(minute.value, 59)
  second.value = normalizePart(second.value, 59)
  emit('update:modelValue', `${year.value}-${pad(month.value + 1)}-${pad(day.value)}T${hour.value}:${minute.value}:${second.value}`)
  closePicker()
}

function clearValue(): void {
  emit('update:modelValue', '')
  closePicker()
}

function normalizePart(value: string, maximum: number): string {
  const digits = value.replace(/\D/g, '')
  const parsed = digits ? Number(digits) : 0
  return pad(Math.min(maximum, Math.max(0, parsed)))
}

function parseLocalDateTime(value: string): Date | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2}))?$/.exec(value)
  if (!match) return null
  const parsed = new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]), Number(match[4]), Number(match[5]), Number(match[6] ?? 0))
  const valid = parsed.getFullYear() === Number(match[1])
    && parsed.getMonth() === Number(match[2]) - 1
    && parsed.getDate() === Number(match[3])
    && parsed.getHours() === Number(match[4])
    && parsed.getMinutes() === Number(match[5])
    && parsed.getSeconds() === Number(match[6] ?? 0)
  return valid ? parsed : null
}

function formatDisplayValue(value: string): string {
  const parsed = parseLocalDateTime(value)
  if (!parsed) return value
  return `${parsed.getFullYear()}/${pad(parsed.getMonth() + 1)}/${pad(parsed.getDate())} ${pad(parsed.getHours())}:${pad(parsed.getMinutes())}:${pad(parsed.getSeconds())}`
}

function sameDate(left: Date, right: Date): boolean {
  return left.getFullYear() === right.getFullYear() && left.getMonth() === right.getMonth() && left.getDate() === right.getDate()
}

function pad(value: number): string {
  return String(value).padStart(2, '0')
}
</script>

<style lang="scss" src="./DateTimeField.scss"></style>
