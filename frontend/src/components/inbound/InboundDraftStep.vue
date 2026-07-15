<!-- 本组件拥有正式入库流程的单据填写步骤；它编辑父页面草稿，但不提交 API 或管理模板请求。 -->
<template>
  <section class="inbound-step inbound-draft-step" aria-labelledby="inbound-draft-step-title">
    <header class="inbound-step__header">
      <div>
        <h2 id="inbound-draft-step-title">填写入库单</h2>
      </div>
      <button class="secondary-button inbound-step-nav-button" type="button" @click="$emit('continue-adding')">上一步：选择物品</button>
    </header>

    <div class="inbound-order__body" :inert="drawerOpen ? true : undefined">
      <section class="inbound-order-meta" aria-label="入库单基础信息">
        <label class="inbound-order-meta__source">
          <span>来源 *</span>
          <input
            data-inbound-source
            :value="source"
            :class="{ 'inbound-control--error': validationAttempted && !source.trim() }"
            :title="validationAttempted && !source.trim() ? '请填写入库来源' : undefined"
            type="text"
            name="inbound_source"
            maxlength="128"
            placeholder="供应商名称或采购单号"
            @input="$emit('update:source', ($event.target as HTMLInputElement).value)"
          />
        </label>
        <button
          class="icon-button inbound-order-meta__notes-toggle"
          :class="{ 'inbound-order-meta__notes-toggle--filled': notes.trim().length > 0 }"
          type="button"
          :title="notesOpen ? '收起备注' : notes.trim() ? '备注已填写' : '添加备注'"
          :aria-label="notesOpen ? '收起备注' : notes.trim() ? '备注已填写' : '添加备注'"
          :aria-expanded="notesOpen"
          aria-controls="inbound-order-notes-v2"
          @click="$emit('update:notes-open', !notesOpen)"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M5 4h14v12H9l-4 4V4Z" />
            <path d="M8 8h8M8 12h5" />
          </svg>
        </button>
        <label v-if="notesOpen" id="inbound-order-notes-v2" class="inbound-order-meta__notes">
          <span>备注</span>
          <input
            :value="notes"
            type="text"
            name="inbound_notes"
            maxlength="1024"
            placeholder="可选，记录采购或收货说明"
            @input="$emit('update:notes', ($event.target as HTMLInputElement).value)"
          />
        </label>
      </section>

      <div v-if="locationError" class="inbound-location-error" role="alert">
        {{ locationError }}
        <button class="text-button" type="button" @click="$emit('retry-locations')">重试</button>
      </div>
      <div v-if="templateOptionsError" class="inbound-template-options-error" role="alert">
        <span>{{ templateOptionsError }}</span>
        <button class="text-button" type="button" :disabled="templateOptionsLoading" @click="$emit('retry-templates')">
          {{ templateOptionsLoading ? '正在重试…' : '重新加载模板' }}
        </button>
      </div>

      <section class="inbound-lines" aria-label="入库明细">
        <table>
          <thead>
            <tr><th>物品</th><th>数量</th><th>单价</th><th>库位</th><th>小计</th><th><span class="visually-hidden">操作</span></th></tr>
          </thead>
          <tbody>
            <tr v-for="(line, index) in lines" :key="line.lineId" :class="{ 'inbound-line--selected': selectedLineId === line.lineId }">
              <td data-label="物品">
                <div class="inbound-line__identity">
                  <AuthenticatedImage :file-id="line.item.image_file_id" :alt="`${line.item.name} 主图`" :size="34" previewable />
                  <div>
                    <strong>{{ line.item.name }}</strong>
                    <span>{{ line.item.sku }} · {{ line.item.unit }} · 明细 {{ index + 1 }}</span>
                    <small class="inbound-line__template-summary" :class="`inbound-line__template-summary--${templateSummary(line).tone}`">
                      <span>入库模板：</span>{{ templateSummary(line).label }}
                      <em v-if="line.templateSource === 'recommended' && line.templateState === 'ready'">已推荐</em>
                    </small>
                  </div>
                </div>
              </td>
              <td data-label="数量"><input v-model.number="line.quantity" :name="`quantity_${line.lineId}`" :data-line-id="line.lineId" data-field="quantity" :class="{ 'inbound-control--error': validationAttempted && !validQuantity(line.quantity) }" type="number" min="0.01" step="0.01" :aria-label="`${line.item.name} 入库数量`" /></td>
              <td data-label="单价"><input v-model.number="line.unitPrice" :name="`unit_price_${line.lineId}`" :data-line-id="line.lineId" data-field="unitPrice" :class="{ 'inbound-control--error': validationAttempted && !validUnitPrice(line.unitPrice) }" type="number" min="0" step="0.01" :aria-label="`${line.item.name} 入库单价`" /></td>
              <td data-label="库位">
                <SelectControl v-model="line.locationId" :name="`location_${line.lineId}`" :data-line-id="line.lineId" data-field="locationId" :aria-invalid="validationAttempted && line.locationId === null ? true : undefined" aria-label="入库库位" compact>
                  <option :value="null">请选择</option>
                  <optgroup v-for="group in locationGroups" :key="group.name" :label="group.name">
                    <option v-for="location in group.locations" :key="location.id" :value="location.id">{{ location.name }}</option>
                  </optgroup>
                </SelectControl>
              </td>
              <td data-label="小计" class="inbound-line__subtotal">¥{{ formatMoney(lineSubtotal(line)) }}</td>
              <td data-label="操作">
                <div class="inbound-line__actions">
                  <button
                    class="inbound-line__completion"
                    :class="`inbound-line__completion--${templateSummary(line).tone}`"
                    type="button"
                    :aria-label="`${line.item.name} ${templateSummary(line).status}，打开批次与入库属性`"
                    @click="$emit('select-line', line.lineId)"
                  >
                    {{ templateSummary(line).status }}
                  </button>
                  <button class="inbound-line__edit" type="button" :data-line-action="line.lineId" @click="$emit('select-line', line.lineId)">批次与入库属性</button>
                  <button
                    class="inbound-line__remove"
                    type="button"
                    :data-line-id="line.lineId"
                    data-field="remove"
                    :aria-label="`移除 ${line.item.name}`"
                    :title="`移除 ${line.item.name}`"
                    @click="$emit('remove-line', line.lineId)"
                  >
                    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                      <path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5" />
                    </svg>
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </section>
    </div>
    <slot />
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { LocationResponse } from '../../api/inbound'
import type { InboundDraftLine } from '../../pages/inbound-draft/model'
import { incompleteTemplateFieldCount, lineSubtotal, validQuantity, validUnitPrice } from '../../pages/inbound-draft/model'
import { formatMoney } from '../../pages/inbound-draft/presentation'
import AuthenticatedImage from '../attributes/AuthenticatedImage.vue'
import SelectControl from '../forms/SelectControl.vue'

const props = defineProps<{
  lines: InboundDraftLine[]
  locations: LocationResponse[]
  locationError: string
  templateOptionsLoading: boolean
  templateOptionsError: string
  source: string
  notes: string
  notesOpen: boolean
  validationAttempted: boolean
  selectedLineId: string | null
  drawerOpen: boolean
}>()

const locationGroups = computed(() => {
  const groups = new Map<string, LocationResponse[]>()
  for (const location of props.locations) {
    const list = groups.get(location.group_name) ?? []
    list.push(location)
    groups.set(location.group_name, list)
  }
  return Array.from(groups, ([name, locations]) => ({ name, locations }))
})

defineEmits<{
  'update:source': [value: string]
  'update:notes': [value: string]
  'update:notes-open': [value: boolean]
  'continue-adding': []
  'retry-locations': []
  'retry-templates': []
  'select-line': [lineId: string]
  'remove-line': [lineId: string]
}>()

function templateSummary(line: InboundDraftLine): { label: string; status: string; tone: 'muted' | 'accent' | 'warning' | 'danger' } {
  if (line.templateState === 'resolving') {
    return {
      label: line.templateSource === 'recommended' ? '正在匹配推荐模板…' : '正在加载…',
      status: '模板加载中',
      tone: 'muted',
    }
  }
  if (line.templateState === 'unresolved') {
    return {
      label: line.templateSource === 'recommended' ? `推荐模板 #${line.templateId} 已失效` : `模板 #${line.templateId} 已失效`,
      status: '需要处理',
      tone: 'warning',
    }
  }
  if (line.templateState === 'error') return { label: '加载失败', status: '需要处理', tone: 'danger' }
  if (!line.template) return { label: '未设置', status: '无需填写', tone: 'muted' }
  const incompleteCount = incompleteTemplateFieldCount(line)
  return {
    label: line.template.name,
    status: incompleteCount > 0 ? `待填写 ${incompleteCount} 项` : '属性已完成',
    tone: incompleteCount > 0 ? 'warning' : 'accent',
  }
}
</script>
