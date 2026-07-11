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
          class="secondary-button inbound-toolbar-button inbound-order-meta__notes-toggle"
          :class="{ 'inbound-order-meta__notes-toggle--filled': notes.trim().length > 0 }"
          type="button"
          :aria-expanded="notesOpen"
          aria-controls="inbound-order-notes-v2"
          @click="$emit('update:notes-open', !notesOpen)"
        >
          {{ notesOpen ? '收起备注' : notes.trim() ? '备注 · 已填写' : '添加备注' }}
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

      <section class="inbound-lines" aria-label="入库明细">
        <table>
          <thead>
            <tr><th>物品</th><th>数量</th><th>单价</th><th>库位</th><th>小计</th><th><span class="visually-hidden">操作</span></th></tr>
          </thead>
          <tbody>
            <tr v-for="(line, index) in lines" :key="line.lineId" :class="{ 'inbound-line--selected': selectedLineId === line.lineId }">
              <td data-label="物品">
                <div class="inbound-line__identity"><AuthenticatedImage :file-id="line.item.image_file_id" :alt="`${line.item.name} 主图`" :size="34" /><div><strong>{{ line.item.name }}</strong><span>{{ line.item.sku }} · {{ line.item.unit }} · 明细 {{ index + 1 }}</span></div></div>
              </td>
              <td data-label="数量"><input v-model.number="line.quantity" :name="`quantity_${line.lineId}`" :data-line-id="line.lineId" data-field="quantity" :class="{ 'inbound-control--error': validationAttempted && !validQuantity(line.quantity) }" type="number" min="0.01" step="0.01" :aria-label="`${line.item.name} 入库数量`" /></td>
              <td data-label="单价"><input v-model.number="line.unitPrice" :name="`unit_price_${line.lineId}`" :data-line-id="line.lineId" data-field="unitPrice" :class="{ 'inbound-control--error': validationAttempted && !validUnitPrice(line.unitPrice) }" type="number" min="0" step="0.01" :aria-label="`${line.item.name} 入库单价`" /></td>
              <td data-label="库位">
                <select v-model="line.locationId" :name="`location_${line.lineId}`" :data-line-id="line.lineId" data-field="locationId" :class="{ 'inbound-control--error': validationAttempted && line.locationId === null }" aria-label="入库库位">
                  <option :value="null">请选择</option>
                  <optgroup v-for="group in locationGroups" :key="group.name" :label="group.name">
                    <option v-for="location in group.locations" :key="location.id" :value="location.id">{{ location.code }} · {{ location.name }}</option>
                  </optgroup>
                </select>
              </td>
              <td data-label="小计" class="inbound-line__subtotal">¥{{ formatMoney(lineSubtotal(line)) }}</td>
              <td data-label="操作">
                <div class="inbound-line__actions">
                  <button class="inbound-line__edit" type="button" :data-line-action="line.lineId" @click="$emit('select-line', line.lineId)">批次与属性</button>
                  <button class="inbound-line__remove" type="button" :aria-label="`移除 ${line.item.name}`" :title="`移除 ${line.item.name}`" @click="$emit('remove-line', line.lineId)">
                    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M6 6l12 12M18 6 6 18" /></svg>
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
import { lineSubtotal, validQuantity, validUnitPrice } from '../../pages/inbound-draft/model'
import { formatMoney } from '../../pages/inbound-draft/presentation'
import AuthenticatedImage from '../attributes/AuthenticatedImage.vue'

const props = defineProps<{
  lines: InboundDraftLine[]
  locations: LocationResponse[]
  locationError: string
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
  'select-line': [lineId: string]
  'remove-line': [lineId: string]
}>()
</script>
