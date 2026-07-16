<!-- 本组件拥有单条审计事件的只读详情、历史差异和原始 JSON；它不重新查询当前业务对象。 -->
<template>
  <ModalDialog
    :open="event !== null"
    :title="dialogTitle"
    description="审计详情来自操作发生时的服务端记录。"
    wide
    @close="emit('close')"
  >
    <template v-if="event" #context>
      <dl class="event-detail-context">
        <div>
          <dt>事件</dt>
          <dd>#{{ event.id }}</dd>
        </div>
        <div>
          <dt>操作时间</dt>
          <dd>{{ formatLocalTimestamp(event.timestamp) }}</dd>
        </div>
        <div>
          <dt>操作人</dt>
          <dd>{{ actorLabel(event) }}</dd>
        </div>
        <div>
          <dt>业务对象</dt>
          <dd>{{ entityTargetLabel(event) }}</dd>
        </div>
      </dl>
    </template>

    <div v-if="event" class="event-detail">
      <section class="event-detail__section">
        <h3>事件信息</h3>
        <dl class="event-detail__metadata">
          <div>
            <dt>原始 UTC</dt>
            <dd>{{ event.timestamp }}</dd>
          </div>
          <div>
            <dt>实体类型</dt>
            <dd>
              <code>{{ event.entity_type }}</code>
            </dd>
          </div>
          <div>
            <dt>动作</dt>
            <dd>
              <code>{{ event.action }}</code>
            </dd>
          </div>
          <div>
            <dt>用户 ID</dt>
            <dd>{{ event.user_id === null ? "无记录" : `#${event.user_id}` }}</dd>
          </div>
        </dl>
      </section>

      <section v-if="permissionChanges" class="event-detail__section">
        <h3>权限变化</h3>
        <div class="event-permission-diff">
          <div>
            <strong>新增权限 · {{ permissionChanges.added.length }}</strong>
            <ul v-if="permissionChanges.added.length">
              <li v-for="permission in permissionChanges.added" :key="permission">
                <code>{{ permission }}</code>
              </li>
            </ul>
            <span v-else>无新增权限</span>
          </div>
          <div>
            <strong>移除权限 · {{ permissionChanges.removed.length }}</strong>
            <ul v-if="permissionChanges.removed.length">
              <li v-for="permission in permissionChanges.removed" :key="permission">
                <code>{{ permission }}</code>
              </li>
            </ul>
            <span v-else>无移除权限</span>
          </div>
        </div>
      </section>

      <section v-if="diffRows.length" class="event-detail__section">
        <h3>字段变化</h3>
        <div class="event-diff-table" role="table" aria-label="审计字段变化">
          <div class="event-diff-table__head" role="row">
            <span role="columnheader">字段</span><span role="columnheader">修改前</span
            ><span role="columnheader">修改后</span>
          </div>
          <div v-for="row in diffRows" :key="row.key" class="event-diff-table__row" role="row">
            <strong role="cell">{{ row.label }}</strong>
            <span role="cell">{{ formatJsonValue(row.previous) }}</span>
            <span role="cell">{{ formatJsonValue(row.next) }}</span>
          </div>
        </div>
      </section>

      <section v-if="previousSnapshot.length" class="event-detail__section">
        <h3>删除前快照</h3>
        <dl class="event-detail__entries">
          <div v-for="entry in previousSnapshot" :key="entry.key">
            <dt>{{ entry.label }}</dt>
            <dd>{{ formatJsonValue(entry.value) }}</dd>
          </div>
        </dl>
      </section>

      <section v-if="detailEntries.length" class="event-detail__section">
        <h3>结构化详情</h3>
        <dl class="event-detail__entries">
          <div v-for="entry in detailEntries" :key="entry.key">
            <dt>{{ entry.label }}</dt>
            <dd>{{ formatJsonValue(entry.value) }}</dd>
          </div>
        </dl>
      </section>

      <details class="event-detail__section event-detail__raw">
        <summary>原始详情</summary>
        <header>
          <span>完整服务端 JSON</span>
          <button class="secondary-button" type="button" @click="copyRawJson">复制 JSON</button>
        </header>
        <pre tabindex="0">{{ rawJson }}</pre>
      </details>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" @click="emit('close')">关闭</button>
      <button
        v-if="event?.entity_id !== null"
        class="primary-button"
        type="button"
        @click="emitRelated"
      >
        查看相关事件
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { EventLogResponse } from "../../api/events";
import { notice } from "../../notices/notice";
import {
  eventDetailEntries,
  eventDiffRows,
  eventPermissionChanges,
  eventPreviousSnapshot,
  formatJsonValue,
  safeJsonStringify,
} from "../../pages/events/details";
import { eventActionLabel, eventEntityLabel } from "../../pages/events/catalog";
import ModalDialog from "../ModalDialog.vue";

const props = defineProps<{ event: EventLogResponse | null }>();
const emit = defineEmits<{ close: []; related: [event: EventLogResponse] }>();

const dialogTitle = computed(() =>
  props.event
    ? `${eventActionLabel(props.event.action)}${eventEntityLabel(props.event.entity_type)}`
    : "审计详情",
);
const diffRows = computed(() => (props.event ? eventDiffRows(props.event.details) : []));
const permissionChanges = computed(() =>
  props.event ? eventPermissionChanges(props.event.details) : null,
);
const previousSnapshot = computed(() =>
  props.event ? eventPreviousSnapshot(props.event.details) : [],
);
const detailEntries = computed(() => (props.event ? eventDetailEntries(props.event.details) : []));
const rawJson = computed(() => (props.event ? safeJsonStringify(props.event.details) : "null"));

function formatLocalTimestamp(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat("zh-CN", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false,
      }).format(date);
}

function actorLabel(event: EventLogResponse): string {
  if (event.username)
    return event.user_id === null ? event.username : `${event.username} · #${event.user_id}`;
  return event.user_id === null ? "系统/未知操作人" : `用户 #${event.user_id}`;
}

function entityTargetLabel(event: EventLogResponse): string {
  const id = event.entity_id === null ? "无实体编号" : `#${event.entity_id}`;
  return `${eventEntityLabel(event.entity_type)} · ${id}`;
}

async function copyRawJson(): Promise<void> {
  try {
    await navigator.clipboard.writeText(rawJson.value);
    notice.success("审计详情 JSON 已复制");
  } catch {
    notice.error("无法复制审计详情 JSON");
  }
}

function emitRelated(): void {
  if (props.event && props.event.entity_id !== null) emit("related", props.event);
}
</script>

<style lang="scss" src="./EventDetailDialog.scss"></style>
