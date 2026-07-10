<!--
  本文件拥有用户列表的搜索、状态筛选和列表级操作工具栏，属于 frontend 用户管理组件层。
  它不请求用户 API，也不拥有分页、权限判断或创建用户流程。
-->
<template>
  <section class="user-list-toolbar" aria-label="用户列表工具栏">
    <form class="user-list-toolbar__filters" role="search" @submit.prevent="emit('apply')">
      <div class="user-list-toolbar__search">
        <label>
          <span class="visually-hidden">搜索用户名</span>
          <input
            :value="search"
            name="search"
            type="search"
            maxlength="64"
            placeholder="搜索用户名"
            @input="updateSearch"
          />
        </label>
        <button class="secondary-button" type="submit" :disabled="loading">搜索</button>
      </div>

      <div class="user-list-toolbar__status" role="group" aria-label="账号状态">
        <button
          v-for="option in statusOptions"
          :key="option.value"
          type="button"
          :class="{ 'user-list-toolbar__status-option--active': status === option.value }"
          :aria-pressed="status === option.value"
          :disabled="loading"
          @click="selectStatus(option.value)"
        >
          {{ option.label }}
        </button>
      </div>

    </form>

    <div class="user-list-toolbar__meta">
      <span class="user-list-toolbar__count">{{ total }} 个用户</span>
      <div class="user-list-toolbar__actions">
        <button class="secondary-button" type="button" :disabled="loading" @click="emit('refresh')">
          {{ loading ? '刷新中…' : '刷新' }}
        </button>
        <button v-if="canRegister" class="primary-button" type="button" @click="emit('create')">
          创建用户
        </button>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { nextTick } from 'vue'
import type { UserStatus } from '../../api/users'

const search = defineModel<string>('search', { required: true })
const status = defineModel<'' | UserStatus>('status', { required: true })

defineProps<{
  total: number
  loading: boolean
  canRegister: boolean
}>()

const emit = defineEmits<{
  apply: []
  refresh: []
  create: []
}>()

const statusOptions: ReadonlyArray<{ label: string; value: '' | UserStatus }> = [
  { label: '全部', value: '' },
  { label: '已启用', value: 'active' },
  { label: '已停用', value: 'disabled' },
]

/** 同步搜索草稿；清空输入时立即恢复未搜索的列表。 */
async function updateSearch(event: Event): Promise<void> {
  const nextSearch = (event.target as HTMLInputElement).value
  search.value = nextSearch

  if (!nextSearch) {
    await nextTick()
    emit('apply')
  }
}

async function selectStatus(nextStatus: '' | UserStatus): Promise<void> {
  if (status.value === nextStatus) {
    return
  }
  status.value = nextStatus
  await nextTick()
  emit('apply')
}

</script>
