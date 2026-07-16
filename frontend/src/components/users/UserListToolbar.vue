<!--
  本文件拥有用户列表的搜索、状态筛选和列表级操作工具栏，属于 frontend 用户管理组件层。
  它不请求用户 API，也不拥有分页、权限判断或创建用户流程。
-->
<template>
  <section class="user-list-toolbar" aria-label="用户列表工具栏">
    <div class="user-list-toolbar__filters">
      <SearchField
        v-model="search"
        class="user-list-toolbar__search"
        label="搜索用户名"
        name="search"
        placeholder="搜索用户名"
        :maxlength="64"
        hide-label
        @search="emit('search', $event)"
      />

      <label class="user-list-toolbar__status">
        <span>账号状态</span>
        <SelectControl
          v-model="status"
          name="user_status"
          :disabled="loading"
          @change="applyStatus"
        >
          <option v-for="option in statusOptions" :key="option.value" :value="option.value">
            {{ option.label }}
          </option>
        </SelectControl>
      </label>
    </div>

    <div class="user-list-toolbar__meta">
      <span class="user-list-toolbar__count">
        <Transition name="user-count" mode="out-in">
          <span :key="total">{{ total }} 个用户</span>
        </Transition>
      </span>
      <div class="user-list-toolbar__actions">
        <span v-if="refreshing" class="user-list-toolbar__refresh-status" aria-hidden="true">
          正在刷新
        </span>
        <button
          class="icon-button user-list-toolbar__refresh"
          :class="{ 'user-list-toolbar__refresh--pending': refreshing }"
          type="button"
          title="刷新用户列表"
          aria-label="刷新用户列表"
          :aria-busy="loading"
          :disabled="loading"
          @click="emit('refresh')"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M20 7v5h-5" />
            <path d="M18.2 16a7 7 0 1 1 .8-7l1 3" />
          </svg>
        </button>
        <span v-if="refreshing" class="visually-hidden" role="status">正在刷新用户列表</span>
        <button
          v-if="canRegister"
          class="icon-button icon-button--primary user-list-toolbar__create"
          type="button"
          title="创建用户"
          aria-label="创建用户"
          @click="emit('create')"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
        </button>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { nextTick } from "vue";
import type { UserStatus } from "../../api/users";
import SearchField from "../SearchField.vue";
import SelectControl from "../forms/SelectControl.vue";

const search = defineModel<string>("search", { required: true });
const status = defineModel<"" | UserStatus>("status", { required: true });

defineProps<{
  total: number;
  loading: boolean;
  refreshing: boolean;
  canRegister: boolean;
}>();

const emit = defineEmits<{
  apply: [];
  search: [value: string];
  refresh: [];
  create: [];
}>();

const statusOptions: ReadonlyArray<{ label: string; value: "" | UserStatus }> = [
  { label: "全部", value: "" },
  { label: "已启用", value: "active" },
  { label: "已停用", value: "disabled" },
];

/** 同步账号状态并立即应用筛选，使选择框行为与其它目录页一致。 */
async function applyStatus(): Promise<void> {
  await nextTick();
  emit("apply");
}
</script>

<style lang="scss" src="./UserListToolbar.scss"></style>
