<!-- 本组件拥有单条审计事件的只读详情、历史差异和原始 JSON；它不重新查询当前业务对象。 -->
<template>
  <ModalDialog
    :open="event !== null"
    :title="dialogTitle"
    :description="$t('events.detailDescription')"
    wide
    @close="emit('close')"
  >
    <template v-if="event" #context>
      <dl class="event-detail-context">
        <div>
          <dt>{{ $t('events.detailEvent') }}</dt>
          <dd>#{{ event.id }}</dd>
        </div>
        <div>
          <dt>{{ $t('events.detailTimestamp') }}</dt>
          <dd>{{ formatLocalTimestamp(event.timestamp) }}</dd>
        </div>
        <div>
          <dt>{{ $t('events.detailActor') }}</dt>
          <dd>{{ actorLabel(event) }}</dd>
        </div>
        <div>
          <dt>{{ $t('events.detailEntity') }}</dt>
          <dd>{{ entityTargetLabel(event) }}</dd>
        </div>
      </dl>
    </template>

    <div v-if="event" class="event-detail">
      <section class="event-detail__section">
        <h3>{{ $t('events.detailEventInfo') }}</h3>
        <dl class="event-detail__metadata">
          <div>
            <dt>{{ $t('events.detailRawUtc') }}</dt>
            <dd>{{ event.timestamp }}</dd>
          </div>
          <div>
            <dt>{{ $t('events.detailEntityType') }}</dt>
            <dd>
              <code>{{ event.entity_type }}</code>
            </dd>
          </div>
          <div>
            <dt>{{ $t('events.detailAction') }}</dt>
            <dd>
              <code>{{ event.action }}</code>
            </dd>
          </div>
          <div>
            <dt>{{ $t('events.detailUserId') }}</dt>
            <dd>{{ event.user_id === null ? $t('events.valueNone') : `#${event.user_id}` }}</dd>
          </div>
        </dl>
      </section>

      <section v-if="permissionChanges" class="event-detail__section">
        <h3>{{ $t('events.permissionChangesTitle') }}</h3>
        <div class="event-permission-diff">
          <div>
            <strong>{{ $t('events.permissionAdded', { n: permissionChanges.added.length }) }}</strong>
            <ul v-if="permissionChanges.added.length">
              <li v-for="permission in permissionChanges.added" :key="permission">
                <code>{{ permission }}</code>
              </li>
            </ul>
            <span v-else>{{ $t('events.permissionAddedNone') }}</span>
          </div>
          <div>
            <strong>{{ $t('events.permissionRemoved', { n: permissionChanges.removed.length }) }}</strong>
            <ul v-if="permissionChanges.removed.length">
              <li v-for="permission in permissionChanges.removed" :key="permission">
                <code>{{ permission }}</code>
              </li>
            </ul>
            <span v-else>{{ $t('events.permissionRemovedNone') }}</span>
          </div>
        </div>
      </section>

      <section v-if="diffRows.length" class="event-detail__section">
        <h3>{{ $t('events.fieldChangesTitle') }}</h3>
        <div v-overlay-scrollbar class="event-diff-table" role="table" :aria-label="$t('events.fieldChangesAria')">
          <div class="event-diff-table__head" role="row">
            <span role="columnheader">{{ $t('events.fieldColumnField') }}</span
            ><span role="columnheader">{{ $t('events.fieldColumnBefore') }}</span
            ><span role="columnheader">{{ $t('events.fieldColumnAfter') }}</span>
          </div>
          <div v-for="row in diffRows" :key="row.key" class="event-diff-table__row" role="row">
            <strong role="cell">{{ row.label }}</strong>
            <span role="cell">{{ formatJsonValue(row.previous) }}</span>
            <span role="cell">{{ formatJsonValue(row.next) }}</span>
          </div>
        </div>
      </section>

      <section v-if="previousSnapshot.length" class="event-detail__section">
        <h3>{{ $t('events.previousSnapshotTitle') }}</h3>
        <dl class="event-detail__entries">
          <div v-for="entry in previousSnapshot" :key="entry.key">
            <dt>{{ entry.label }}</dt>
            <dd>{{ formatJsonValue(entry.value) }}</dd>
          </div>
        </dl>
      </section>

      <section v-if="detailEntries.length" class="event-detail__section">
        <h3>{{ $t('events.structuredDetailsTitle') }}</h3>
        <dl class="event-detail__entries">
          <div v-for="entry in detailEntries" :key="entry.key">
            <dt>{{ entry.label }}</dt>
            <dd>{{ formatJsonValue(entry.value) }}</dd>
          </div>
        </dl>
      </section>

      <details class="event-detail__section event-detail__raw">
        <summary>{{ $t('events.rawDetailsTitle') }}</summary>
        <header>
          <span>{{ $t('events.rawJsonLabel') }}</span>
          <button
            v-copyable="{ text: rawJson, label: $t('events.copyJsonLabel') }"
            class="secondary-button"
            type="button"
          >
            {{ $t('events.copyJson') }}
          </button>
        </header>
        <pre v-overlay-scrollbar tabindex="0">{{ rawJson }}</pre>
      </details>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" @click="emit('close')">{{ $t('common.close') }}</button>
      <button
        v-if="event?.entity_id !== null"
        class="primary-button"
        type="button"
        @click="emitRelated"
      >
        {{ $t('events.viewRelatedEvents') }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { EventLogResponse } from "../../api/events";
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
const { t } = useI18n();

const dialogTitle = computed(() =>
  props.event
    ? t("events.detailTitleWithEntity", {
        action: eventActionLabel(props.event.action),
        entity: eventEntityLabel(props.event.entity_type),
      })
    : t("events.detailTitle"),
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
  return event.user_id === null
    ? t("events.systemUnknownActor")
    : t("events.userActor", { id: event.user_id });
}

function entityTargetLabel(event: EventLogResponse): string {
  const id = event.entity_id === null ? t("events.noEntityId") : `#${event.entity_id}`;
  return `${eventEntityLabel(event.entity_type)} · ${id}`;
}

function emitRelated(): void {
  if (props.event && props.event.entity_id !== null) emit("related", props.event);
}
</script>

<style lang="scss" src="./EventDetailDialog.scss"></style>
