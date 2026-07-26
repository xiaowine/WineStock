<!--
  本组件拥有入库草稿"批量设置库位"Dialog：为所有仍未选择库位的明细一次指定库位。
  它不修改草稿行（确认后由入库装配应用），也不加载库位数据。
-->
<template>
  <ModalDialog
    :open="open"
    title="批量设置库位"
    :description="`将为 ${count} 条尚未选择库位的明细设置同一库位；已选库位的明细不受影响。`"
    compact
    @close="emit('close')"
  >
    <label class="inbound-batch-location__field">
      <span>入库库位</span>
      <SelectControl v-model="locationId" name="inbound_batch_location" compact>
        <option :value="null">请选择</option>
        <optgroup v-for="group in locationGroups" :key="group.name" :label="group.name">
          <option v-for="location in group.locations" :key="location.id" :value="location.id">
            {{ location.name }}{{ location.is_default ? "（默认）" : "" }}
          </option>
        </optgroup>
      </SelectControl>
    </label>
    <template #actions>
      <button class="secondary-button" type="button" @click="emit('close')">取消</button>
      <button
        class="primary-button"
        type="button"
        :disabled="locationId === null"
        @click="confirm"
      >
        应用到 {{ count }} 条明细
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { LocationResponse } from "../../api/inbound";
import ModalDialog from "../ModalDialog.vue";
import SelectControl from "../forms/SelectControl.vue";

const props = defineProps<{
  open: boolean;
  /** 仍未选择库位的明细数。 */
  count: number;
  locations: LocationResponse[];
}>();

const emit = defineEmits<{
  close: [];
  confirm: [locationId: number];
}>();

const locationId = ref<number | null>(null);

watch(
  () => props.open,
  (open) => {
    if (open) locationId.value = null;
  },
);

const locationGroups = computed(() => {
  const groups = new Map<string, LocationResponse[]>();
  for (const location of props.locations) {
    const list = groups.get(location.group_name) ?? [];
    list.push(location);
    groups.set(location.group_name, list);
  }
  return Array.from(groups, ([name, locations]) => ({ name, locations }));
});

function confirm(): void {
  if (locationId.value !== null) emit("confirm", locationId.value);
}
</script>

<style scoped lang="scss">
.inbound-batch-location__field {
  display: grid;
  gap: 6px;

  > span {
    color: var(--color-muted);
    font-size: 13px;
  }
}
</style>
