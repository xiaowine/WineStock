<template>
  <section class="route-page inbound-orders-page">
    <header class="content-header inbound-orders-page__header">
      <div>
        <h1>{{ $title($route.meta.title) }}</h1>
        <p>{{ $t("orders.outboundSubtitle") }}</p>
      </div>
      <button
        v-if="canCreate"
        class="primary-button"
        type="button"
        @click="router.push({ name: 'outbound' })"
      >
        {{ $t("orders.createOutbound") }}
      </button>
    </header>
    <section class="inbound-orders-workspace" :aria-label="$t('orders.outboundListLabel')">
      <div class="inbound-orders-toolbar">
        <SearchField
          v-model="searchInput"
          :label="$t('orders.searchOutbound')"
          name="outbound_order_search"
          :placeholder="$t('orders.searchOutboundPlaceholder')"
          hide-label
          @search="applySearch"
        />
        <div class="inbound-orders-toolbar__meta">
          <span class="inbound-orders-count">{{ $t("orders.count", { n: total }) }}</span>
          <div class="inbound-orders-toolbar__actions">
            <button
              class="icon-button inbound-orders-toolbar__filter"
              type="button"
              :title="$t('orders.filterOutbound')"
              :aria-expanded="filterOpen"
              @click="filterOpen = true"
            >
              <svg viewBox="0 0 24 24"><path d="M4 6h16M7 12h10M10 18h4" /></svg
              ><span v-if="filterCount">{{ filterCount }}</span></button
            ><button
              class="icon-button inbound-orders-toolbar__refresh"
              type="button"
              :title="$t('orders.refreshOutbound')"
              :disabled="pending"
              @click="loadFirst"
            >
              <svg viewBox="0 0 24 24">
                <path d="M20 7v5h-5" />
                <path d="M18.2 16a7 7 0 1 1 .8-7l1 3" />
              </svg>
            </button>
            <button
              v-if="canCreate"
              class="icon-button icon-button--primary inbound-orders-toolbar__create"
              type="button"
              :title="$t('orders.createOutbound')"
              :aria-label="$t('orders.createOutbound')"
              @click="router.push({ name: 'outbound' })"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
            </button>
          </div>
        </div>
      </div>
      <div v-overlay-scrollbar class="inbound-orders-results" :aria-busy="pending">
        <section v-if="!orders.length && loaded" class="inbound-orders-state">
          <strong>{{
            hasFilters ? $t("orders.noOutboundFilteredResults") : $t("orders.noOutboundResults")
          }}</strong
          ><button v-if="hasFilters" class="text-button" type="button" @click="clear">
            {{ $t("orders.clearFilters") }}
          </button>
        </section>
        <template v-else-if="!error || loaded"
          ><div class="inbound-orders-table" role="table">
            <div class="inbound-orders-table__head" role="row">
              <span>{{ $t("orders.documentAndDestination") }}</span
              ><span>{{ $t("orders.outboundItems") }}</span
              ><span>{{ $t("orders.statusAndActions") }}</span>
            </div>
            <article
              v-for="o in orders"
              :key="o.id"
              class="inbound-orders-table__row"
              role="row"
              tabindex="0"
              @click="open(o)"
              @keydown.enter="open(o)"
            >
              <div>
                <strong>{{ $t("orders.outboundOrderTitle", { id: o.id }) }}</strong
                ><span>{{ o.destination }}</span
                ><time>{{ date(o.created_at) }}</time>
              </div>
              <div>
                <div class="inbound-order-item-summary">
                  <AuthenticatedImage
                    :file-id="o.items[0].item_image_file_id"
                    :alt="$t('orders.mainImageAlt', { name: o.items[0].item_name })"
                    :size="34"
                    previewable
                    @click.stop
                    @keydown.stop
                  />
                  <div>
                    <strong>{{ o.items[0].item_name }}</strong
                    ><small
                      >{{ o.items[0].item_sku }} · {{ o.items[0].quantity }}
                      {{ o.items[0].item_unit
                      }}{{ o.items.length > 1 ? $t("orders.andMoreItems", { n: o.items.length }) : "" }}</small
                    >
                  </div>
                </div>
                <span>{{ $t("orders.itemsCount", { n: o.items.length }) }}</span>
              </div>
              <div class="inbound-orders-table__decision">
                <span class="inbound-status" :class="`inbound-status--${o.status}`">{{
                  label(o.status)
                }}</span
                ><time v-if="statusTime(o)">{{ date(statusTime(o)!) }}</time
                ><button
                  class="icon-button"
                  type="button"
                  :title="$t('orders.viewOutboundDetail')"
                  @click.stop="open(o)"
                >
                  <svg viewBox="0 0 24 24">
                    <circle cx="12" cy="12" r="8" />
                    <path d="M12 11v5M12 8h.01" />
                  </svg>
                </button>
              </div>
            </article>
          </div>
          <div class="inbound-orders-mobile-list" role="list">
            <article
              v-for="o in orders"
              :key="`mobile-${o.id}`"
              class="inbound-orders-mobile-list__item"
              role="listitem"
              tabindex="0"
              @click="open(o)"
              @keydown.enter="open(o)"
              @keydown.space.prevent="open(o)"
            >
              <header>
                <strong>{{ $t("orders.outboundOrderTitle", { id: o.id }) }}</strong>
                <span class="inbound-status" :class="`inbound-status--${o.status}`">{{
                  label(o.status)
                }}</span>
              </header>
              <p class="inbound-orders-mobile-list__source">{{ o.destination }}</p>
              <time :datetime="o.created_at">{{ date(o.created_at) }}</time>
              <div class="inbound-orders-mobile-list__item-summary">
                <AuthenticatedImage
                  :file-id="o.items[0].item_image_file_id"
                  :alt="$t('orders.mainImageAlt', { name: o.items[0].item_name })"
                  :size="38"
                  previewable
                  @click.stop
                  @keydown.stop
                />
                <div>
                  <strong>{{ o.items[0].item_name }}</strong>
                  <small
                    >{{ o.items[0].item_sku }} · {{ o.items[0].quantity }} {{ o.items[0].item_unit
                    }}{{ o.items.length > 1 ? $t("orders.andMoreItems", { n: o.items.length }) : "" }}</small
                  >
                </div>
              </div>
              <div class="inbound-orders-mobile-list__metrics">
                <span>{{ $t("orders.itemsCount", { n: o.items.length }) }}</span>
                <strong>{{ statusTime(o) ? date(statusTime(o)!) : $t("orders.stockNotDeducted") }}</strong>
              </div>
            </article>
          </div>
          <div ref="sentinel" class="inbound-orders-load-more">
            <span v-if="moreLoading">{{ $t("orders.loadingMoreOutbound") }}</span
            ><button v-else-if="moreError" class="secondary-button" @click="next">
              {{ $t("orders.loadMoreFailedRetry") }}</button
            ><span v-else-if="more">{{ $t("orders.scrollToLoad") }}</span
            ><span v-else>{{ $t("orders.loadedAllOutbound", { n: total }) }}</span>
          </div></template
        >
      </div>
    </section>
    <OutboundOrderFiltersDialog
      :open="filterOpen"
      :value="filterValue"
      @close="filterOpen = false"
      @apply="apply"
    /><ModalDialog
      :open="selected !== null"
      wide
      :title="selected ? $t('orders.outboundOrderTitle', { id: selected.id }) : ''"
      @close="selected = null"
      ><template #context
        ><div v-if="selected" class="dialog-account-context">
          <span>{{ label(selected.status) }}</span
          ><strong>{{ selected.destination }}</strong>
        </div></template
      >
      <template v-if="selected"
        ><dl class="inbound-detail-summary">
          <div>
            <dt>{{ $t("orders.createdAt") }}</dt>
            <dd>{{ date(selected.created_at) }}</dd>
          </div>
          <div>
            <dt>{{ $t("orders.status") }}</dt>
            <dd>{{ statusDescription(selected) }}</dd>
          </div>
          <div>
            <dt>{{ $t("orders.remark") }}</dt>
            <dd>{{ selected.notes || $t("orders.noRemark") }}</dd>
          </div>
        </dl>
        <section class="inbound-detail-items">
          <h3>
            {{ $t("orders.outboundItems") }}
            <span>{{ $t("orders.itemCount", { n: selected.items.length }) }}</span>
          </h3>
          <article v-for="i in selected.items" :key="i.id">
            <header class="inbound-detail-item__header">
              <AuthenticatedImage
                :file-id="i.item_image_file_id"
                :alt="$t('orders.mainImageAlt', { name: i.item_name })"
                :size="52"
                previewable
              />
              <div>
                <strong v-copyable="{ text: i.item_name, label: $t('orders.itemName') }">{{
                  i.item_name
                }}</strong
                ><small
                  ><span v-copyable="{ text: i.item_sku, label: $t('orders.itemSkuLabel') }">{{
                    i.item_sku
                  }}</span>
                  · {{ i.item_unit }} · {{ $t("orders.itemRef", { id: i.item_id }) }}</small
                >
              </div>
              <span>{{ i.quantity }} {{ i.item_unit }}</span>
            </header>
            <dl>
              <div>
                <dt>{{ $t("orders.location") }}</dt>
                <dd>{{ i.location_name || $t("orders.allLocations") }}</dd>
              </div>
              <div>
                <dt>{{ $t("orders.deductionMethod") }}</dt>
                <dd>
                  {{
                    i.batch_id
                      ? $t("orders.specifiedBatch", { id: i.batch_id })
                      : $t("orders.fifoAllocation")
                  }}
                </dd>
              </div>
            </dl>
          </article>
        </section></template
      ><template #actions
        ><button class="secondary-button" type="button" @click="selected = null">
          {{ $t("orders.close") }}
        </button
        ><button
          v-if="canApprove && selected?.status === 'pending'"
          class="primary-button"
          @click="router.push({ name: 'outbound-approvals' })"
        >
          {{ $t("orders.goToOutboundApprovals") }}
        </button></template
      ></ModalDialog
    >
  </section>
</template>
<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import {
  getOutboundOrder,
  listOutboundOrders,
  type OutboundOrderResponse,
  type OutboundOrderStatus,
} from "../api/outboundOrders";
import { hasPermission, stockPermissions } from "../auth/permissions";
import { authSession } from "../auth/session";
import AuthenticatedImage from "../components/attributes/AuthenticatedImage.vue";
import ModalDialog from "../components/ModalDialog.vue";
import OutboundOrderFiltersDialog, {
  type OutboundOrderFilterValue,
} from "../components/outbound/OutboundOrderFiltersDialog.vue";
import SearchField from "../components/SearchField.vue";
import { notice } from "../notices/notice";
import "../components/inbound/InboundOrderList.scss";
import "./InboundOrdersPage.scss";
const { t } = useI18n();
const route = useRoute(),
  router = useRouter(),
  orders = ref<OutboundOrderResponse[]>([]),
  total = ref(0),
  pages = ref(0),
  loaded = ref(false),
  loading = ref(false),
  moreLoading = ref(false),
  error = ref(""),
  moreError = ref(""),
  selected = ref<OutboundOrderResponse | null>(null),
  detailError = ref(""),
  searchInput = ref(""),
  from = ref(""),
  to = ref(""),
  filterOpen = ref(false),
  sentinel = ref<HTMLElement | null>(null),
  state = reactive({ page: 1, status: "" as OutboundOrderStatus | "", search: "" });
let ctrl: AbortController | null = null,
  observer: IntersectionObserver | null = null;
const pending = computed(() => loading.value || moreLoading.value),
  more = computed(() => state.page < pages.value),
  canCreate = computed(() =>
    hasPermission(authSession.value?.user.permissions, stockPermissions.outboundCreate),
  ),
  canApprove = computed(() =>
    hasPermission(authSession.value?.user.permissions, stockPermissions.outboundApprove),
  ),
  hasFilters = computed(() => !!(state.search || state.status || from.value || to.value)),
  filterCount = computed(() => [state.status, from.value, to.value].filter(Boolean).length),
  filterValue = computed<OutboundOrderFilterValue>(() => ({
    status: state.status,
    dateFrom: local(from.value),
    dateTo: local(to.value),
  }));
watch(
  () => route.fullPath,
  () => {
    state.status = isStatus(route.query.status) ? route.query.status : "";
    state.search = typeof route.query.search === "string" ? route.query.search : "";
    searchInput.value = state.search;
    from.value = iso(route.query.date_from);
    to.value = iso(route.query.date_to);
    void loadFirst();
  },
  { immediate: true },
);
onMounted(() => {
  observer = new IntersectionObserver(
    (e) => {
      if (e.some((x) => x.isIntersecting)) void next();
    },
    { rootMargin: "240px 0px" },
  );
  watch(
    sentinel,
    () => {
      if (sentinel.value) {
        observer?.disconnect();
        observer?.observe(sentinel.value);
      }
    },
    { immediate: true },
  );
});
onBeforeUnmount(() => {
  ctrl?.abort();
  observer?.disconnect();
});
async function load(p = 1, append = false) {
  ctrl?.abort();
  const c = new AbortController();
  ctrl = c;
  append ? (moreLoading.value = true) : (loading.value = true);
  append ? (moreError.value = "") : (error.value = "");
  try {
    const r = await listOutboundOrders(
      {
        page: p,
        page_size: 50,
        search: state.search || undefined,
        status: state.status || undefined,
        date_from: from.value || undefined,
        date_to: to.value || undefined,
      },
      c.signal,
    );
    if (ctrl !== c) return;
    orders.value = append
      ? [...orders.value, ...r.items.filter((i) => !orders.value.some((x) => x.id === i.id))]
      : r.items;
    total.value = r.total;
    pages.value = r.total_pages;
    state.page = r.page;
    loaded.value = true;
  } catch (e) {
    if (!(e instanceof DOMException && e.name === "AbortError")) {
      if (append) {
        moreError.value = t("orders.loadMoreFailed");
      } else {
        error.value = t("orders.connectionCheckRetry");
        notice.error(loaded.value ? t("orders.refreshOutboundFailed") : t("orders.loadOutboundFailed"), {
          detail: error.value,
          onClick: () => void loadFirst(),
        });
      }
    }
  } finally {
    if (ctrl === c) {
      ctrl = null;
      loading.value = false;
      moreLoading.value = false;
      void nextTick();
    }
  }
}
function loadFirst() {
  return load();
}
function next() {
  if (!pending.value && more.value) return load(state.page + 1, true);
}
function query(v: Record<string, string | undefined>) {
  void router.replace({ query: v });
}
function applySearch(v: string) {
  query({
    search: v.trim() || undefined,
    status: state.status || undefined,
    date_from: from.value || undefined,
    date_to: to.value || undefined,
  });
}
function apply(v: OutboundOrderFilterValue) {
  const a = toIso(v.dateFrom),
    b = toIso(v.dateTo);
  if ((v.dateFrom && !a) || (v.dateTo && !b) || (a && b && a > b)) {
    error.value = t("orders.invalidDateRange");
    return;
  }
  filterOpen.value = false;
  query({
    search: state.search || undefined,
    status: v.status || undefined,
    date_from: a || undefined,
    date_to: b || undefined,
  });
}
function clear() {
  query({});
}
async function open(o: OutboundOrderResponse) {
  selected.value = o;
  detailError.value = "";
  try {
    selected.value = await getOutboundOrder(o.id);
  } catch (error) {
    if (!(error instanceof DOMException && error.name === "AbortError")) {
      detailError.value = t("orders.outboundDetailLoadFailed");
      notice.error(t("orders.outboundDetailLoadFailedTitle"), {
        detail: detailError.value,
        onClick: retrySelected,
      });
    }
  }
}
function retrySelected(): void {
  const order = selected.value;
  if (order) void open(order);
}
function isStatus(v: unknown): v is OutboundOrderStatus {
  return v === "pending" || v === "approved" || v === "rejected";
}
function iso(v: unknown) {
  return typeof v === "string" && !Number.isNaN(new Date(v).getTime())
    ? new Date(v).toISOString()
    : "";
}
function local(v: string) {
  if (!v) return "";
  const d = new Date(v);
  return new Date(d.getTime() - d.getTimezoneOffset() * 60000).toISOString().slice(0, 19);
}
function toIso(v: string) {
  return v && !Number.isNaN(new Date(v).getTime()) ? new Date(v).toISOString() : "";
}
function label(s: OutboundOrderStatus) {
  return {
    pending: t("orders.statusPending"),
    approved: t("orders.statusApprovedOutbound"),
    rejected: t("orders.statusRejected"),
  }[s];
}
function statusTime(o: OutboundOrderResponse) {
  return o.status === "approved" ? o.approved_at : o.status === "rejected" ? o.rejected_at : null;
}
function statusDescription(o: OutboundOrderResponse) {
  return o.status === "pending"
    ? t("orders.waitingApprovalNoDeduct")
    : o.status === "approved"
      ? t("orders.approvedOutboundDesc", { time: date(o.approved_at || "") })
      : t("orders.rejectedNoDeduct");
}
function date(v: string) {
  const d = new Date(v);
  return Number.isNaN(d.getTime())
    ? v
    : new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(d);
}
</script>
