<!--
  本组件拥有可跨业务页面复用的物品新建 Dialog，组合共享编辑器和新建会话。
  它负责关闭确认与创建结果事件，不拥有调用方的目录刷新或业务草稿。
-->
<template>
  <ItemEditorDialog
    :open="open"
    mode="create"
    :draft="draft"
    :categories="categories"
    :templates="templates"
    :saving="saving"
    :metadata-error="metadataError"
    :validation-errors="validationErrors"
    :auto-lcsc-code="initialLcscCode"
    @save="saveItem"
    @close="requestClose"
    @apply-lcsc="applyLcscCandidate"
  />

  <ModalDialog
    :open="discardDialogOpen"
    title="放弃新建物品？"
    description="当前填写的物品资料不会保留。"
    @close="cancelDiscard"
  >
    <p>确认后返回原来的业务页面。</p>
    <template #actions>
      <button class="secondary-button" type="button" @click="cancelDiscard">继续编辑</button>
      <button class="danger-button" type="button" @click="confirmClose">放弃新建</button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { onBeforeRouteLeave } from "vue-router";
import type { ItemOptionResponse } from "../../api/items";
import ModalDialog from "../ModalDialog.vue";
import ItemEditorDialog from "./ItemEditorDialog.vue";
import { useItemCreateSession } from "./useItemCreateSession";

const props = withDefaults(
  defineProps<{
    open: boolean;
    /** 打开时自动以该 C 号拉取立创资料预填（入库扫码未命中路径）。 */
    initialLcscCode?: string;
  }>(),
  { initialLcscCode: "" },
);
const emit = defineEmits<{
  close: [];
  created: [item: ItemOptionResponse];
}>();

const discardDialogOpen = ref(false);
let pendingLeaveResolution: ((allowed: boolean) => void) | null = null;
const {
  draft,
  categories,
  templates,
  saving,
  metadataError,
  validationErrors,
  hasUnsavedChanges,
  loadMetadata,
  applyLcscCandidate,
  save,
  discard,
} = useItemCreateSession();

watch(
  () => props.open,
  (open) => {
    if (open) void loadMetadata();
  },
);

onMounted(() => window.addEventListener("beforeunload", handleBeforeUnload));
onBeforeUnmount(() => {
  window.removeEventListener("beforeunload", handleBeforeUnload);
  pendingLeaveResolution?.(false);
  pendingLeaveResolution = null;
  void discard();
});

onBeforeRouteLeave(() => {
  if (!props.open || !hasUnsavedChanges.value) return true;
  discardDialogOpen.value = true;
  return new Promise<boolean>((resolve) => {
    pendingLeaveResolution = resolve;
  });
});

async function saveItem(): Promise<void> {
  const item = await save();
  if (!item) return;
  await discard();
  emit("created", item);
}

function requestClose(): void {
  if (!props.open || saving.value) return;
  if (hasUnsavedChanges.value) {
    discardDialogOpen.value = true;
    return;
  }
  void closeAfterDiscard();
}

async function confirmClose(): Promise<void> {
  discardDialogOpen.value = false;
  await discard();
  if (pendingLeaveResolution) {
    const resolve = pendingLeaveResolution;
    pendingLeaveResolution = null;
    resolve(true);
    return;
  }
  emit("close");
}

async function closeAfterDiscard(): Promise<void> {
  await discard();
  emit("close");
}

function cancelDiscard(): void {
  discardDialogOpen.value = false;
  pendingLeaveResolution?.(false);
  pendingLeaveResolution = null;
}

function handleBeforeUnload(event: BeforeUnloadEvent): void {
  if (!props.open || !hasUnsavedChanges.value) return;
  event.preventDefault();
  event.returnValue = "";
}
</script>
