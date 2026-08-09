<!--
  本组件拥有全局替代关系组的三段式和移动呈现；它只展示服务端真实字段并发出查看事件。
-->
<template>
  <article
    class="substitute-relation-group"
    role="row"
    tabindex="0"
    :aria-label="$t('substitutes.viewRelationsAria', { name: group.itemName })"
    @click="emit('open', group)"
    @keydown.enter.self="emit('open', group)"
    @keydown.space.self.prevent="emit('open', group)"
  >
    <div class="substitute-relation-group__identity" role="cell">
      <span class="substitute-relation-group__direction">{{ $t('substitutes.directionLabel') }}</span>
      <strong :title="group.itemName">{{ group.itemName }}</strong>
      <dl>
        <div>
          <dt>{{ $t('substitutes.skuLabel') }}</dt>
          <dd :title="group.itemSku">{{ group.itemSku }}</dd>
        </div>
        <div>
          <dt>{{ $t('substitutes.itemIdLabel') }}</dt>
          <dd>#{{ group.itemId }}</dd>
        </div>
      </dl>
    </div>

    <div class="substitute-relation-group__summary" role="cell">
      <div class="substitute-relation-group__first">
        <span>{{ $t('substitutes.primarySubstitute') }}</span>
        <strong :title="group.firstSubstitute.substitute_item_name">{{
          group.firstSubstitute.substitute_item_name
        }}</strong>
        <small :title="group.firstSubstitute.substitute_item_sku">
          {{ $t('substitutes.priorityAndSku', { priority: group.firstSubstitute.priority, sku: group.firstSubstitute.substitute_item_sku }) }}
        </small>
      </div>
      <div class="substitute-relation-group__chips" :aria-label="$t('substitutes.summaryLabel')">
        <span>{{ $t('substitutes.substituteCount', { n: group.relations.length }) }}</span>
        <span v-if="group.hasNotes">{{ $t('substitutes.hasNotes') }}</span>
      </div>
    </div>

    <div class="substitute-relation-group__decision" role="cell">
      <span
        ><strong>{{ group.relations.length }}</strong> {{ $t('substitutes.relationUnit') }}</span
      >
      <button
        class="icon-button"
        type="button"
        :title="$t('substitutes.viewRelations')"
        :aria-label="$t('substitutes.viewRelationsAria', { name: group.itemName })"
        @click.stop="emit('open', group)"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <circle cx="12" cy="12" r="8" />
          <path d="M12 10v6M12 7.5v.5" />
        </svg>
      </button>
    </div>
  </article>
</template>

<script setup lang="ts">
import type { SubstituteRelationGroupModel } from "../../pages/substitutes/model";

defineProps<{ group: SubstituteRelationGroupModel }>();

const emit = defineEmits<{
  open: [group: SubstituteRelationGroupModel];
}>();
</script>

<style lang="scss" src="./SubstituteRelationGroup.scss"></style>
