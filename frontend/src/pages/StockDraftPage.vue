<!--
  本文件拥有"新建入库/新建出库"共用的草稿页：按路由 kind 装配同一工作台壳的入库或出库领域。
  领域字段、校验与提交由 pages/stock-draft/ 的装配注入；本页不直接实现业务规则。
-->
<template>
  <StockDraftWorkspace
    v-if="inbound"
    :flow="inbound.flow"
    :texts="inboundDraftTexts"
    :handle="handle"
    :can-create-item="inbound.canCreateItem.value"
  >
    <template #summary>
      <span
        ><strong>{{
          $t('stockDraft.linesSummary', { n: inbound.flow.lines.value.length })
        }}</strong></span
      >
      <span aria-hidden="true">·</span>
      <span
        >{{ $t('stockDraft.inboundQuantity') }}
        <strong>{{ inbound.quantitySummary.value }}</strong></span
      >
      <template v-if="inbound.draftAmountReady.value">
        <span aria-hidden="true">·</span>
        <span
          >{{ $t('stockDraft.expectedAmount') }}
          <strong>¥{{ formatMoney(inbound.draftTotal.value) }}</strong></span
        >
      </template>
    </template>

    <template #actions>
      <div
        ref="importMenuRoot"
        class="inbound-import-actions"
        @keydown.esc.stop="closeImportMenu(true)"
      >
        <button
          class="secondary-button inbound-add-item-button inbound-import-action--desktop"
          type="button"
          :title="$t('stockDraft.importOrderTitle')"
          @click="openOrderImport"
        >
          {{ $t('stockDraft.importOrder') }}
        </button>
        <button
          v-if="inbound.canCreateItem.value"
          class="secondary-button inbound-add-item-button inbound-import-action--desktop"
          type="button"
          :title="$t('stockDraft.importBackupTitle')"
          @click="openBackupImport"
        >
          {{ $t('stockDraft.importBackup') }}
        </button>
        <button
          ref="importMenuTrigger"
          class="icon-button inbound-import-menu__trigger"
          type="button"
          :title="$t('stockDraft.importInboundData')"
          :aria-label="$t('stockDraft.importInboundData')"
          :aria-expanded="importMenuOpen"
          :aria-controls="importMenuId"
          @click="toggleImportMenu"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 3v12M8 11l4 4 4-4M5 20h14" />
          </svg>
        </button>
        <Transition name="inbound-import-menu">
          <div
            v-if="importMenuOpen"
            :id="importMenuId"
            class="inbound-import-menu"
            role="group"
            :aria-label="$t('stockDraft.chooseImportMethod')"
          >
            <button type="button" @click="openOrderImport">
              {{ $t('stockDraft.importOrder') }}
            </button>
            <button v-if="inbound.canCreateItem.value" type="button" @click="openBackupImport">
              {{ $t('stockDraft.importBackup') }}
            </button>
          </div>
        </Transition>
      </div>
      <button
        v-if="inbound.pendingLocationCount.value > 0"
        class="secondary-button inbound-add-item-button inbound-batch-location-button"
        type="button"
        :title="$t('stockDraft.batchSetLocationTitle')"
        @click="inbound.batchLocationOpen.value = true"
      >
        {{ $t('stockDraft.batchSetLocation') }}
      </button>
    </template>

    <template #line-cells="{ line }">
      <td :data-label="$t('common.amount')">
        <strong
          class="inbound-line__value"
          :class="{ 'inbound-line__value--warning': !validQuantity(line.quantity) }"
        >
          {{ inboundQuantityLabel(line) }}
        </strong>
      </td>
      <td :data-label="$t('stockDraft.unitPriceSubtotal')">
        <div class="inbound-line__value-stack">
          <strong
            class="inbound-line__value"
            :class="{ 'inbound-line__value--warning': !validUnitPrice(line.unitPrice) }"
          >
            {{ inboundPriceLabel(line) }}
          </strong>
          <span v-if="validQuantity(line.quantity) && validUnitPrice(line.unitPrice)">
            {{ $t('stockDraft.subtotal') }} ¥{{ formatMoney(lineSubtotal(line)) }}
          </span>
        </div>
      </td>
      <td :data-label="$t('stockDraft.location')">
        <strong
          class="inbound-line__value inbound-line__value--truncate"
          :class="{
            'inbound-line__value--warning': line.locationId === null,
            'inbound-line__value--danger':
              line.locationId !== null && !inboundLocationMap.has(line.locationId),
          }"
          :title="inboundLocationLabel(line)"
        >
          {{ inboundLocationLabel(line) }}
        </strong>
      </td>
      <td :data-label="$t('stockDraft.batch')">
        <span class="inbound-line__value--truncate" :title="inboundBatchDetail(line)">
          {{ inboundBatchDetail(line) }}
        </span>
      </td>
    </template>

    <template #line-editor="{ line }">
      <div v-if="inbound.scanOrderPrompt.value" class="inbound-scan-source-prompt" role="status">
        <span>{{
          $t('stockDraft.scanOrderPrompt', { order: inbound.scanOrderPrompt.value })
        }}</span>
        <button class="text-button" type="button" @click="inbound.applyScanOrderNo">
          {{ $t('stockDraft.fillIn') }}
        </button>
        <button class="text-button" type="button" @click="inbound.ignoreScanOrderNo">
          {{ $t('stockDraft.ignore') }}
        </button>
        <button class="text-button" type="button" @click="inbound.suppressScanOrderPrompt">
          {{ $t('stockDraft.dontAskAgain') }}
        </button>
      </div>
      <InboundLineEditor
        :line="line"
        :locations="inbound.locations.value"
        :location-error="inbound.locationError.value"
        :validation-attempted="inbound.flow.validationAttempted.value"
        @retry-locations="inbound.loadLocationOptions"
      />
    </template>

    <template #submit-summary>
      <dl class="inbound-submit-summary">
        <div>
          <dt>{{ $t('stockDraft.inboundSource') }}</dt>
          <dd>{{ inbound.flow.source.value.trim() }}</dd>
        </div>
        <div>
          <dt>{{ $t('stockDraft.lineCount') }}</dt>
          <dd>{{ $t('stockDraft.linesCountShort', { n: inbound.flow.lines.value.length }) }}</dd>
        </div>
        <div>
          <dt>{{ $t('stockDraft.inboundTotalQuantity') }}</dt>
          <dd>{{ inbound.quantitySummary.value }}</dd>
        </div>
        <div>
          <dt>{{ $t('stockDraft.expectedAmount') }}</dt>
          <dd>¥{{ formatMoney(inbound.draftTotal.value) }}</dd>
        </div>
      </dl>
      <p v-if="inbound.flow.canDirect.value" class="inbound-submit-warning">
        {{ $t('stockDraft.directInboundWarning') }}
      </p>
    </template>

    <template #extras>
      <ItemCreateDialog
        :open="inbound.itemCreateOpen.value"
        :initial-lcsc-code="inbound.scanLcscCode.value"
        @close="inbound.handleItemCreateClosed"
        @created="inbound.handleItemCreated"
      />
      <LcscOrderImportDialog
        :open="inbound.orderImportOpen.value"
        :existing-skus="inboundDraftSkus"
        :can-create-item="inbound.canCreateItem.value"
        :source-filled="inbound.flow.source.value.trim().length > 0"
        @close="inbound.orderImportOpen.value = false"
        @import="inbound.importOrderLines"
      />
      <InboundBatchLocationDialog
        :open="inbound.batchLocationOpen.value"
        :count="inbound.pendingLocationCount.value"
        :locations="inbound.locations.value"
        @close="inbound.batchLocationOpen.value = false"
        @confirm="inbound.applyBatchLocation"
      />
      <ErpBackupImportDialog
        :open="inbound.backupImportOpen.value"
        :can-create-item="inbound.canCreateItem.value"
        @close="inbound.backupImportOpen.value = false"
        @import="inbound.importBackup"
      />
    </template>
  </StockDraftWorkspace>

  <section
    v-else-if="outbound && !outbound.canReadItems.value"
    class="route-page outbound-draft-page"
  >
    <section class="outbound-blocked">
      <h2>{{ $t('stockDraft.outboundBlockedTitle') }}</h2>
      <p>{{ $t('stockDraft.outboundBlockedBody') }}</p>
      <button class="secondary-button" type="button" @click="router.back()">
        {{ $t('common.back') }}
      </button>
    </section>
  </section>

  <StockDraftWorkspace
    v-else-if="outbound"
    :flow="outbound.flow"
    :texts="outboundDraftTexts"
    :handle="handle"
  >
    <template #summary>
      <span
        ><strong>{{
          $t('stockDraft.linesSummary', { n: outbound.flow.lines.value.length })
        }}</strong></span
      >
      <span aria-hidden="true">·</span>
      <span
        >{{ $t('stockDraft.outboundQuantity') }}
        <strong>{{ outbound.quantitySummary.value }}</strong></span
      >
      <template v-if="outbound.costSummary.value.state === 'complete'">
        <span aria-hidden="true">·</span>
        <span
          >{{ $t('stockDraft.expectedCost') }}
          <strong>¥{{ formatMoney(outbound.costSummary.value.amount ?? 0) }}</strong></span
        >
      </template>
      <template v-else-if="outbound.costSummary.value.state === 'loading'">
        <span aria-hidden="true">·</span><span>{{ $t('stockDraft.estimatingCost') }}</span>
      </template>
      <template
        v-else-if="
          outbound.costSummary.value.state === 'insufficient' ||
          outbound.costSummary.value.state === 'failed'
        "
      >
        <span aria-hidden="true">·</span><span>{{ $t('stockDraft.costPerActualOutbound') }}</span>
      </template>
    </template>

    <template #line-cells="{ line }">
      <td :data-label="$t('common.amount')">
        <strong
          class="outbound-line__value"
          :class="{ 'outbound-line__value--warning': !outbound.validQuantity(line.quantity) }"
        >
          {{ outbound.quantityLabel(line) }}
        </strong>
      </td>
      <td :data-label="$t('stockDraft.allocationBatch')">
        <div class="outbound-line__value-stack">
          <strong class="outbound-line__value">{{ outbound.allocationPrimary(line) }}</strong>
          <span
            v-if="outbound.allocationSecondary(line)"
            class="outbound-line__value--truncate"
            :class="{
              'outbound-line__value--warning':
                line.allocationMode === 'specific_batch' && line.batchId === null,
              'outbound-line__value--danger': outbound.batchUnavailable(line),
            }"
            :title="outbound.allocationSecondary(line)"
          >
            {{ outbound.allocationSecondary(line) }}
          </span>
        </div>
      </td>
      <td :data-label="$t('stockDraft.location')">
        <strong
          class="outbound-line__value outbound-line__value--truncate"
          :title="outbound.allocationLocationLabel(line)"
        >
          {{ outbound.allocationLocationLabel(line) }}
        </strong>
      </td>
      <td :data-label="$t('stockDraft.expectedCost')">
        <div class="outbound-line__value-stack">
          <strong
            class="outbound-line__value outbound-line__value--truncate"
            :class="{
              'outbound-line__value--warning':
                outbound.lineCostEstimate(line).state === 'insufficient',
            }"
            :title="outbound.costEstimatePrimary(line)"
          >
            {{ outbound.costEstimatePrimary(line) }}
          </strong>
          <span v-if="outbound.costEstimateSecondary(line)">
            {{ outbound.costEstimateSecondary(line) }}
          </span>
        </div>
      </td>
    </template>

    <template #line-editor="{ line }">
      <OutboundAllocationEditor
        :line="line"
        :draft="outbound.allocationDraft"
        :batches="outbound.batches.value"
        :batch-error="outbound.batchError.value"
        :batch-pending="outbound.batchPending.value"
        :batch-more="outbound.batchMore.value"
        :locations="outbound.locations.value"
        :location-error="outbound.locationError.value"
        :validation="outbound.flow.validationAttempted.value"
        :cost-hint="outbound.allocationCostHint.value"
        @retry-batches="outbound.resetBatches"
        @load-more-batches="outbound.loadBatches"
      />
    </template>

    <template #submit-summary>
      <dl class="outbound-confirm">
        <div>
          <dt>{{ $t('stockDraft.outboundDestination') }}</dt>
          <dd>{{ outbound.flow.source.value }}</dd>
        </div>
        <div>
          <dt>{{ $t('stockDraft.detailsLabel') }}</dt>
          <dd>
            {{
              $t('stockDraft.linesAndQuantity', {
                n: outbound.flow.lines.value.length,
                q: outbound.quantitySummary.value,
              })
            }}
          </dd>
        </div>
        <div>
          <dt>{{ $t('stockDraft.expectedOutboundCost') }}</dt>
          <dd>{{ outbound.confirmCostLabel.value }}</dd>
        </div>
      </dl>
    </template>
  </StockDraftWorkspace>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, useId } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import StockDraftWorkspace from "../components/stock-draft/StockDraftWorkspace.vue";
import LcscOrderImportDialog from "../components/stock-draft/LcscOrderImportDialog.vue";
import OutboundAllocationEditor from "../components/stock-draft/OutboundAllocationEditor.vue";
import InboundBatchLocationDialog from "../components/inbound/InboundBatchLocationDialog.vue";
import InboundLineEditor from "../components/inbound/InboundLineEditor.vue";
import ErpBackupImportDialog from "../components/stock-draft/ErpBackupImportDialog.vue";
import ItemCreateDialog from "../components/items/ItemCreateDialog.vue";
import { useNativeBackHandler } from "../composables/useNativeBackHandler";
import { NativeBackPriority } from "../navigation/nativeBack";
import {
  lineSubtotal,
  validQuantity,
  validUnitPrice,
  type InboundDraftLine,
} from "./inbound-draft/model";
import { formatMoney, formatQuantity } from "./inbound-draft/presentation";
import { createWorkspaceHandle } from "./stock-draft/flow";
import { inboundDraftTexts, useInboundDraft } from "./stock-draft/useInboundDraft";
import { outboundDraftTexts, useOutboundDraft } from "./stock-draft/useOutboundDraft";
// 工作台骨架沿用 inbound-* 类名，出库单元格与分配编辑器使用 outbound-* 类名；两份域样式均归本页所有。
import "./stock-draft/inbound.scss";
import "./stock-draft/outbound.scss";

const props = defineProps<{ kind: "inbound" | "outbound" }>();

const { t } = useI18n();
const router = useRouter();
// 路由出口按路由 name 重建组件实例，kind 在单个实例生命周期内恒定，
// 因此按 kind 条件装配领域 composable 是安全的。
const handle = createWorkspaceHandle();
const inbound = props.kind === "inbound" ? useInboundDraft(handle) : null;
const outbound = props.kind === "outbound" ? useOutboundDraft(handle) : null;
const importMenuRoot = ref<HTMLElement | null>(null);
const importMenuTrigger = ref<HTMLButtonElement | null>(null);
const importMenuOpen = ref(false);
const importMenuId = `inbound-import-menu-${useId()}`;

useNativeBackHandler({
  id: `stock-draft-import-menu:${importMenuId}`,
  active: importMenuOpen,
  priority: NativeBackPriority.TransientOverlay,
  handle: () => {
    if (!importMenuOpen.value) return { handled: false };
    closeImportMenu(true);
    return { handled: true, reason: "transient-overlay" };
  },
});

onBeforeUnmount(removeImportMenuListener);

const inboundLocationMap = computed(
  () => new Map((inbound?.locations.value ?? []).map((location) => [location.id, location])),
);

// 订单导入按 SKU（立创 C 号）预排除已在草稿中的物品。
const inboundDraftSkus = computed<ReadonlySet<string>>(
  () =>
    new Set((inbound?.flow.lines.value ?? []).map((line) => line.item.sku.trim().toUpperCase())),
);

function toggleImportMenu(): void {
  importMenuOpen.value ? closeImportMenu(true) : void showImportMenu();
}

async function showImportMenu(): Promise<void> {
  importMenuOpen.value = true;
  window.addEventListener("pointerdown", handleImportMenuOutsidePointer);
  await nextTick();
  importMenuRoot.value?.querySelector<HTMLElement>(".inbound-import-menu button")?.focus();
}

function closeImportMenu(restoreFocus = false): void {
  if (!importMenuOpen.value) return;
  importMenuOpen.value = false;
  removeImportMenuListener();
  if (restoreFocus) void nextTick(() => importMenuTrigger.value?.focus());
}

function handleImportMenuOutsidePointer(event: PointerEvent): void {
  if (importMenuRoot.value?.contains(event.target as Node)) return;
  closeImportMenu();
}

function removeImportMenuListener(): void {
  window.removeEventListener("pointerdown", handleImportMenuOutsidePointer);
}

function openOrderImport(): void {
  closeImportMenu();
  if (inbound) inbound.orderImportOpen.value = true;
}

function openBackupImport(): void {
  closeImportMenu();
  if (inbound) inbound.backupImportOpen.value = true;
}

function inboundQuantityLabel(line: InboundDraftLine): string {
  return validQuantity(line.quantity)
    ? formatQuantity(line.quantity) + " " + line.item.unit
    : t("stockDraft.toBeFilled");
}

function inboundPriceLabel(line: InboundDraftLine): string {
  return validUnitPrice(line.unitPrice) ? "¥" + formatMoney(line.unitPrice) : t("stockDraft.toBeFilled");
}

function inboundLocationLabel(line: InboundDraftLine): string {
  return line.locationId === null
    ? t("stockDraft.toBeSelected")
    : (inboundLocationMap.value.get(line.locationId)?.name ?? t("stockDraft.locationInvalid"));
}

function inboundBatchDetail(line: InboundDraftLine): string {
  const batch = line.batchNo.trim() || t("stockDraft.autoGeneratedBatch");
  const expiry = line.expiresAt || t("stockDraft.noExpiry");
  return `${batch} · ${expiry}`;
}
</script>
