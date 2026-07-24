<!-- 本组件拥有入库单据基本信息与明细复核区域；它编辑父页面草稿，但不提交 API 或管理模板请求。 -->
<template>
  <section class="inbound-step inbound-draft-step" aria-labelledby="inbound-draft-step-title">
    <header class="inbound-step__header">
      <div>
        <h2 id="inbound-draft-step-title">入库单信息与明细</h2>
        <p>添加后立即配置该物品；未完成明细会锁定下一次添加。</p>
      </div>
      <button
        class="primary-button inbound-add-item-button"
        type="button"
        :disabled="!canAddItem"
        :title="canAddItem ? '选择物品并配置明细' : addItemDisabledReason"
        @click="$emit('add-item')"
      >
        添加物品
      </button>
    </header>

    <p v-if="!canAddItem && addItemDisabledReason" class="inbound-add-item-hint" role="status">
      {{ addItemDisabledReason }}
    </p>

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
        <button
          class="text-button"
          type="button"
          :disabled="templateOptionsLoading"
          @click="$emit('retry-templates')"
        >
          {{ templateOptionsLoading ? "正在重试…" : "重新加载模板" }}
        </button>
      </div>

      <section v-if="lines.length === 0" class="inbound-panel-state inbound-lines-empty">
        <strong>还没有入库明细</strong>
        <span>点击“添加物品”选择一项，完成对应入库明细。</span>
      </section>

      <section v-else class="inbound-lines" aria-label="入库明细">
        <table>
          <thead>
            <tr>
              <th>物品</th>
              <th>本次入库</th>
              <th>模板与批次</th>
              <th><span class="visually-hidden">操作</span></th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="line in lines"
              :key="line.lineId"
              :class="{ 'inbound-line--selected': selectedLineId === line.lineId }"
            >
              <td data-label="物品">
                <div class="inbound-line__identity">
                  <AuthenticatedImage
                    :file-id="line.item.image_file_id"
                    :alt="line.item.name + ' 主图'"
                    :size="34"
                    previewable
                  />
                  <div>
                    <strong :title="line.item.name">{{ line.item.name }}</strong>
                    <span>{{ line.item.sku }} · {{ line.item.unit }}</span>
                  </div>
                </div>
              </td>
              <td data-label="本次入库">
                <div class="inbound-line__summary">
                  <span>数量：{{ quantityLabel(line) }}</span>
                  <span>单价：{{ priceLabel(line) }}</span>
                  <span>库位：{{ locationLabel(line) }}</span>
                  <strong v-if="validQuantity(line.quantity) && validUnitPrice(line.unitPrice)">
                    小计 ¥{{ formatMoney(lineSubtotal(line)) }}
                  </strong>
                </div>
              </td>
              <td data-label="模板与批次">
                <div class="inbound-line__summary inbound-line__summary--secondary">
                  <small
                    class="inbound-line__template-summary"
                    :class="'inbound-line__template-summary--' + templateSummary(line).tone"
                  >
                    <span>入库模板：</span>{{ templateSummary(line).label }}
                    <em
                      v-if="line.templateSource === 'recommended' && line.templateState === 'ready'"
                      >已推荐</em
                    >
                  </small>
                  <span>批次：{{ line.batchNo.trim() || "由服务端生成" }}</span>
                  <span>有效期：{{ line.expiresAt || "未设置" }}</span>
                </div>
              </td>
              <td data-label="操作">
                <div class="inbound-line__actions">
                  <span
                    v-if="lineStatusLabel(line)"
                    class="inbound-line__status"
                    :class="'inbound-line__status--' + lineStatusTone(line)"
                  >
                    {{ lineStatusLabel(line) }}
                  </span>
                  <button
                    class="icon-button inbound-line__edit"
                    type="button"
                    :data-line-action="line.lineId"
                    :aria-label="line.item.name + '，编辑入库明细'"
                    :title="'编辑 ' + line.item.name"
                    @click="$emit('select-line', line.lineId)"
                  >
                    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                      <path d="m5 17-1 3 3-1L19 7l-2-2L5 17Z" />
                      <path d="m15 7 2 2" />
                    </svg>
                  </button>
                  <button
                    class="icon-button inbound-line__remove"
                    type="button"
                    :data-line-id="line.lineId"
                    data-field="remove"
                    :aria-label="'移除 ' + line.item.name"
                    :title="'移除 ' + line.item.name"
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
import { computed } from "vue";
import type { LocationResponse } from "../../api/inbound";
import type { InboundDraftLine } from "../../pages/inbound-draft/model";
import {
  incompleteTemplateFieldCount,
  lineSubtotal,
  validQuantity,
  validUnitPrice,
} from "../../pages/inbound-draft/model";
import { formatMoney, formatQuantity } from "../../pages/inbound-draft/presentation";
import AuthenticatedImage from "../attributes/AuthenticatedImage.vue";

const props = defineProps<{
  lines: InboundDraftLine[];
  locations: LocationResponse[];
  locationError: string;
  templateOptionsLoading: boolean;
  templateOptionsError: string;
  source: string;
  notes: string;
  notesOpen: boolean;
  validationAttempted: boolean;
  selectedLineId: string | null;
  drawerOpen: boolean;
  canAddItem: boolean;
  addItemDisabledReason: string;
}>();

defineEmits<{
  "update:source": [value: string];
  "update:notes": [value: string];
  "update:notes-open": [value: boolean];
  "retry-locations": [];
  "retry-templates": [];
  "select-line": [lineId: string];
  "remove-line": [lineId: string];
  "add-item": [];
}>();

const locationMap = computed(() => new Map(props.locations.map((location) => [location.id, location])));

function quantityLabel(line: InboundDraftLine): string {
  return validQuantity(line.quantity) ? formatQuantity(line.quantity) + " " + line.item.unit : "待填写";
}

function priceLabel(line: InboundDraftLine): string {
  return validUnitPrice(line.unitPrice) ? "¥" + formatMoney(line.unitPrice) : "待填写";
}

function locationLabel(line: InboundDraftLine): string {
  return line.locationId === null
    ? "待选择"
    : (locationMap.value.get(line.locationId)?.name ?? "库位已失效");
}

function templateSummary(line: InboundDraftLine): {
  label: string;
  status: string;
  tone: "muted" | "accent" | "warning" | "danger";
} {
  if (line.templateState === "resolving")
    return {
      label: line.templateSource === "recommended" ? "正在匹配推荐模板…" : "正在加载…",
      status: "模板加载中",
      tone: "muted",
    };
  if (line.templateState === "unresolved")
    return {
      label:
        line.templateSource === "recommended"
          ? "推荐模板已失效"
          : "所选模板已失效",
      status: "需要处理",
      tone: "warning",
    };
  if (line.templateState === "error")
    return { label: "加载失败", status: "需要处理", tone: "danger" };
  if (!line.template) return { label: "未设置", status: "无需填写", tone: "muted" };
  const incompleteCount = incompleteTemplateFieldCount(line);
  return {
    label: line.template.name,
    status: incompleteCount > 0 ? "待填写 " + incompleteCount + " 项" : "属性已完成",
    tone: incompleteCount > 0 ? "warning" : "accent",
  };
}

function lineStatusLabel(line: InboundDraftLine): string | null {
  if (!validQuantity(line.quantity)) return "待填写数量";
  if (!validUnitPrice(line.unitPrice)) return "待填写单价";
  if (line.locationId === null) return "待选择库位";
  if (line.templateState === "resolving") return "模板加载中";
  if (line.templateState === "unresolved" || line.templateState === "error")
    return "需要处理";
  if (line.template && incompleteTemplateFieldCount(line) > 0) return "待补充属性";
  return null;
}

function lineStatusTone(line: InboundDraftLine): "muted" | "warning" | "danger" {
  if (line.templateState === "error") return "danger";
  if (
    !validQuantity(line.quantity) ||
    !validUnitPrice(line.unitPrice) ||
    line.locationId === null ||
    line.templateState === "unresolved" ||
    (line.template !== null && incompleteTemplateFieldCount(line) > 0)
  )
    return "warning";
  if (line.templateState === "resolving") return "muted";
  return "muted";
}
</script>
