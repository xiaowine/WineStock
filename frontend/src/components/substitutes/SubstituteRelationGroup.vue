<!--
  本组件拥有全局替代关系组的三段式和移动呈现；它只展示服务端真实字段并发出查看事件。
-->
<template>
  <article
    class="substitute-relation-group"
    role="row"
    tabindex="0"
    :aria-label="`查看 ${group.itemName} 的替代关系`"
    @click="emit('open', group)"
    @keydown.enter.self="emit('open', group)"
    @keydown.space.self.prevent="emit('open', group)"
  >
    <div class="substitute-relation-group__identity" role="cell">
      <span class="substitute-relation-group__direction">主物品 → 替代物品</span>
      <strong :title="group.itemName">{{ group.itemName }}</strong>
      <dl>
        <div>
          <dt>编号</dt>
          <dd :title="group.itemSku">{{ group.itemSku }}</dd>
        </div>
        <div>
          <dt>物品 ID</dt>
          <dd>#{{ group.itemId }}</dd>
        </div>
      </dl>
    </div>

    <div class="substitute-relation-group__summary" role="cell">
      <div class="substitute-relation-group__first">
        <span>首选替代</span>
        <strong :title="group.firstSubstitute.substitute_item_name">{{
          group.firstSubstitute.substitute_item_name
        }}</strong>
        <small :title="group.firstSubstitute.substitute_item_sku">
          优先级 {{ group.firstSubstitute.priority }} · 编号
          {{ group.firstSubstitute.substitute_item_sku }}
        </small>
      </div>
      <div class="substitute-relation-group__chips" aria-label="关系摘要">
        <span>{{ group.relations.length }} 个替代项</span>
        <span v-if="group.hasNotes">有兼容性说明</span>
      </div>
    </div>

    <div class="substitute-relation-group__decision" role="cell">
      <span
        ><strong>{{ group.relations.length }}</strong> 条关系</span
      >
      <button
        class="icon-button"
        type="button"
        title="查看替代关系"
        :aria-label="`查看 ${group.itemName} 的替代关系`"
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
