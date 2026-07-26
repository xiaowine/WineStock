<!-- 本组件按业务域呈现删除后果与影响范围的普通确认，不调用删除 API，也不要求输入内容验证。 -->
<template>
  <ModalDialog
    :open="Boolean(target)"
    :title="dialogTitle"
    :busy="submitting"
    @close="emit('close')"
  >
    <template #context>
      <div v-if="target" class="dialog-account-context dialog-account-context--danger">
        <span>删除目标</span>
        <strong>{{ target.name }}</strong>
      </div>
    </template>
    <div class="template-delete-copy">
      <template v-if="target?.kind === 'category'">
        <p>分类将从活动列表移除。已有物品可能继续保存该分类 ID，但不会自动改到其他分类。</p>
      </template>
      <template v-else-if="target">
        <p><strong>此操作会造成物品属性数据丢失：</strong></p>
        <ul>
          <li>已有物品与该模板的关联会被解除。</li>
          <li>由该模板字段定义的物品属性值会被删除。</li>
          <li>此操作无法从当前界面恢复。</li>
        </ul>
      </template>
      <section v-if="target" class="template-delete-copy__impact" aria-label="删除影响范围">
        <strong>影响范围</strong>
        <p v-if="target.itemUsageCount !== null && target.itemUsageCount > 0">
          当前有 {{ target.itemUsageCount }} 个有效物品使用此{{
            target.kind === "category" ? "分类" : "模板"
          }}。
        </p>
        <p v-else>当前没有有效物品使用此{{ target.kind === "category" ? "分类" : "模板" }}。</p>
      </section>
      <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
    </div>
    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        取消
      </button>
      <button class="danger-button" type="button" :disabled="submitting" @click="emit('submit')">
        {{ submitting ? "正在删除…" : target?.kind === "item" ? "删除模板及属性" : "确认删除" }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { TemplateDomain } from "../../pages/templates/model";
import ModalDialog from "../ModalDialog.vue";

export interface TemplateDeleteTarget {
  id: number;
  name: string;
  kind: TemplateDomain;
  /** 打开确认框时从列表响应读取的当前有效物品使用数量。 */
  itemUsageCount: number | null;
}

const props = defineProps<{
  target: TemplateDeleteTarget | null;
  submitting: boolean;
  errorMessage: string;
}>();

const emit = defineEmits<{
  close: [];
  submit: [];
}>();

const dialogTitle = computed(() =>
  props.target?.kind === "category" ? "删除物品分类" : "删除物品属性模板",
);
</script>
