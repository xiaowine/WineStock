<!--
  本文件拥有用户管理页面的数据加载、筛选分页和管理操作编排，属于 frontend 页面层。
  它只通过 HTTP API 管理用户，不直接访问 token、数据库或后端内部业务对象。
-->
<template>
  <section class="route-page users-page">
    <header class="content-header users-page__header">
      <div>
        <h1>用户管理</h1>
        <p>查看账号状态，并按权限执行创建、启停、删除、权限分配和临时密码操作。</p>
      </div>
    </header>

    <UserListToolbar
      v-model:search="searchInput"
      v-model:status="statusInput"
      :total="total"
      :loading="loading"
      :can-register="canRegister"
      @apply="applyFilters"
      @refresh="refreshUsers"
      @create="openCreateDialog"
    />

    <section v-if="loadError" class="page-state page-state--error" role="alert">
      <h2>无法加载用户</h2>
      <p>{{ loadError }}</p>
      <button class="secondary-button" type="button" @click="loadUsers">重试</button>
    </section>

    <section v-else class="users-content" :aria-busy="loading">
      <div v-if="loading && users.length === 0" class="page-state" role="status">
        正在加载用户…
      </div>

      <div v-else-if="users.length === 0" class="page-state">
        <h2>没有符合条件的用户</h2>
        <p>{{ hasActiveFilters ? '调整搜索或状态筛选后重试。' : '当前服务还没有可显示的用户。' }}</p>
      </div>

      <template v-else>
        <div class="users-table-wrap">
          <table class="users-table">
            <colgroup>
              <col class="users-table__user-column" />
              <col class="users-table__status-column" />
              <col class="users-table__permissions-column" />
              <col class="users-table__password-column" />
              <col class="users-table__updated-column" />
              <col class="users-table__actions-column" />
            </colgroup>
            <thead>
              <tr>
                <th scope="col">用户</th>
                <th scope="col">状态</th>
                <th scope="col">权限</th>
                <th scope="col">密码</th>
                <th scope="col">最近更新</th>
                <th scope="col" class="users-table__actions-heading">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="user in users" :key="user.id">
                <td>
                  <span class="user-identity-cell">
                    <strong :title="user.username">{{ user.username }}</strong>
                    <small>#{{ user.id }}<template v-if="isCurrentUser(user)"> · 当前账号</template></small>
                  </span>
                </td>
                <td>
                  <span class="status-pill" :class="statusClass(user.status)">
                    {{ statusLabel(user.status) }}
                  </span>
                </td>
                <td>{{ user.permissions.length }} 项</td>
                <td>
                  <span
                    class="status-pill"
                    :class="user.password_change_required ? 'status-pill--warn' : 'status-pill--neutral'"
                  >
                    {{ user.password_change_required ? '待修改' : '正常' }}
                  </span>
                </td>
                <td>{{ formatDate(user.updated_at) }}</td>
                <td>
                  <div class="row-actions">
                    <button
                      v-if="canEditPermissions"
                      class="text-button"
                      type="button"
                      @click="openPermissionsDialog(user)"
                    >
                      权限
                    </button>
                    <button
                      v-if="canResetPassword && !isCurrentUser(user)"
                      class="text-button"
                      type="button"
                      @click="openPasswordDialog(user)"
                    >
                      临时密码
                    </button>
                    <button
                      v-if="canUpdateStatus && !isCurrentUser(user)"
                      class="text-button"
                      type="button"
                      @click="openStatusDialog(user)"
                    >
                      {{ user.status === 'active' ? '停用' : '启用' }}
                    </button>
                    <button
                      v-if="canDelete && !isCurrentUser(user)"
                      class="text-button"
                      type="button"
                      @click="openDeleteDialog(user)"
                    >
                      删除
                    </button>
                    <span v-if="!hasAvailableAction(user)" class="row-actions__empty">无可用操作</span>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="users-mobile-list">
          <article v-for="user in users" :key="user.id" class="user-mobile-item">
            <header>
              <div>
                <h2 :title="user.username">{{ user.username }}</h2>
                <p>#{{ user.id }}<template v-if="isCurrentUser(user)"> · 当前账号</template></p>
              </div>
              <span class="status-pill" :class="statusClass(user.status)">
                {{ statusLabel(user.status) }}
              </span>
            </header>
            <dl>
              <div>
                <dt>权限</dt>
                <dd>{{ user.permissions.length }} 项</dd>
              </div>
              <div>
                <dt>密码状态</dt>
                <dd>{{ user.password_change_required ? '待修改临时密码' : '正常' }}</dd>
              </div>
              <div>
                <dt>最近更新</dt>
                <dd>{{ formatDate(user.updated_at) }}</dd>
              </div>
            </dl>
            <div v-if="hasAvailableAction(user)" class="row-actions row-actions--mobile">
              <button
                class="secondary-button"
                type="button"
                @click="openActionsDialog(user)"
              >
                管理操作
              </button>
            </div>
          </article>
        </div>

        <nav v-if="totalPages > 1" class="pagination" aria-label="用户列表分页">
          <button
            class="secondary-button"
            type="button"
            :disabled="page <= 1 || loading"
            @click="goToPage(page - 1)"
          >
            上一页
          </button>
          <span>第 {{ page }} / {{ totalPages }} 页</span>
          <button
            class="secondary-button"
            type="button"
            :disabled="page >= totalPages || loading"
            @click="goToPage(page + 1)"
          >
            下一页
          </button>
        </nav>
      </template>
    </section>

    <UserCreateDialog
      :open="createDialogOpen"
      :submitting="actionSubmitting"
      :error-message="actionError"
      @close="closeDialogs"
      @submit="createUser"
    />
    <UserPermissionsDialog
      :user="permissionsUser"
      :permissions="permissionDefinitions"
      :loading="permissionsLoading"
      :load-error="permissionsLoadError"
      :submitting="actionSubmitting"
      :error-message="actionError"
      :editing-current-user="permissionsUser ? isCurrentUser(permissionsUser) : false"
      @close="closeDialogs"
      @retry="loadPermissionDefinitions"
      @submit="savePermissions"
    />
    <UserPasswordResetDialog
      :user="passwordUser"
      :submitting="actionSubmitting"
      :error-message="actionError"
      @close="closeDialogs"
      @submit="saveTemporaryPassword"
    />
    <UserStatusDialog
      :user="statusUser"
      :next-status="nextStatus"
      :submitting="actionSubmitting"
      :error-message="actionError"
      @close="closeDialogs"
      @submit="saveStatus"
    />
    <UserDeleteDialog
      :user="deleteUserTarget"
      :submitting="actionSubmitting"
      :error-message="actionError"
      @close="closeDialogs"
      @submit="confirmDeleteUser"
    />
    <UserActionsDialog
      :user="actionsUser"
      :can-edit-permissions="canEditPermissions"
      :can-reset-password="Boolean(actionsUser && canResetPassword && !isCurrentUser(actionsUser))"
      :can-update-status="Boolean(actionsUser && canUpdateStatus && !isCurrentUser(actionsUser))"
      :can-delete="Boolean(actionsUser && canDelete && !isCurrentUser(actionsUser))"
      @close="closeDialogs"
      @permissions="selectPermissionsAction"
      @password="selectPasswordAction"
      @status="selectStatusAction"
      @delete="selectDeleteAction"
    />
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  deleteUser,
  listPermissions,
  listUsers,
  registerUser,
  resetUserPassword,
  updateUserPermissions,
  updateUserStatus,
  type PermissionResponse,
  type UserAdminResponse,
  type UserStatus,
} from '../api/users'
import {
  ApiConfigurationError,
  ApiError,
  ApiNetworkError,
  ApiResponseError,
} from '../api/errors'
import { hasPermission, userPermissions } from '../auth/permissions'
import { authSession, replaceCurrentSessionPermissions } from '../auth/session'
import UserActionsDialog from '../components/users/UserActionsDialog.vue'
import UserCreateDialog from '../components/users/UserCreateDialog.vue'
import UserDeleteDialog from '../components/users/UserDeleteDialog.vue'
import UserListToolbar from '../components/users/UserListToolbar.vue'
import UserPasswordResetDialog from '../components/users/UserPasswordResetDialog.vue'
import UserPermissionsDialog from '../components/users/UserPermissionsDialog.vue'
import UserStatusDialog from '../components/users/UserStatusDialog.vue'
import { notice } from '../notices/notice'

const PAGE_SIZE = 20

const router = useRouter()
const users = ref<UserAdminResponse[]>([])
const total = ref(0)
const page = ref(1)
const totalPages = ref(0)
const searchInput = ref('')
const statusInput = ref<'' | UserStatus>('')
const activeSearch = ref('')
const activeStatus = ref<'' | UserStatus>('')
const loading = ref(false)
const loadError = ref('')
const createDialogOpen = ref(false)
const permissionsUser = ref<UserAdminResponse | null>(null)
const passwordUser = ref<UserAdminResponse | null>(null)
const statusUser = ref<UserAdminResponse | null>(null)
const deleteUserTarget = ref<UserAdminResponse | null>(null)
const actionsUser = ref<UserAdminResponse | null>(null)
const nextStatus = ref<UserStatus>('disabled')
const actionSubmitting = ref(false)
const actionError = ref('')
const permissionDefinitions = ref<PermissionResponse[]>([])
const permissionsLoading = ref(false)
const permissionsLoadError = ref('')
let usersAbortController: AbortController | null = null
let permissionsAbortController: AbortController | null = null

const currentPermissions = computed(() => authSession.value?.user.permissions)
const currentUserId = computed(() => authSession.value?.user.id)
const canRegister = computed(() =>
  hasPermission(currentPermissions.value, userPermissions.register),
)
const canUpdateStatus = computed(() =>
  hasPermission(currentPermissions.value, userPermissions.updateStatus),
)
const canDelete = computed(() =>
  hasPermission(currentPermissions.value, userPermissions.delete),
)
const canResetPassword = computed(() =>
  hasPermission(currentPermissions.value, userPermissions.resetPassword),
)
const canEditPermissions = computed(
  () =>
    hasPermission(currentPermissions.value, userPermissions.updatePermissions) &&
    hasPermission(currentPermissions.value, userPermissions.readPermissionDefinitions),
)
const hasActiveFilters = computed(
  () => Boolean(activeSearch.value) || Boolean(activeStatus.value),
)

onMounted(loadUsers)
onBeforeUnmount(() => {
  usersAbortController?.abort()
  permissionsAbortController?.abort()
})

/** 查询用户列表；新请求会取消旧请求，避免慢响应覆盖最新筛选结果。 */
async function loadUsers(): Promise<void> {
  usersAbortController?.abort()
  const controller = new AbortController()
  usersAbortController = controller
  loading.value = true
  loadError.value = ''

  try {
    const response = await listUsers(
      {
        page: page.value,
        page_size: PAGE_SIZE,
        search: activeSearch.value || undefined,
        status: activeStatus.value || undefined,
      },
      controller.signal,
    )
    users.value = response.items
    total.value = response.total
    totalPages.value = response.total_pages
    page.value = response.page
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') {
      return
    }
    loadError.value = userManagementErrorMessage(error, '加载用户失败')
    notice.error(loadError.value)
  } finally {
    if (usersAbortController === controller) {
      usersAbortController = null
      loading.value = false
    }
  }
}

/** 手动刷新用户列表；与自动加载区分，仅手动成功时反馈。 */
async function refreshUsers(): Promise<void> {
  await loadUsers()
  if (!loadError.value) {
    notice.success('用户列表已刷新')
  }
}

function applyFilters(): void {
  activeSearch.value = searchInput.value.trim()
  activeStatus.value = statusInput.value
  page.value = 1
  void loadUsers()
}

function goToPage(nextPage: number): void {
  page.value = nextPage
  void loadUsers()
}

function openCreateDialog(): void {
  closeDialogs()
  createDialogOpen.value = true
}

function openPermissionsDialog(user: UserAdminResponse): void {
  closeDialogs()
  permissionsUser.value = user
  if (permissionDefinitions.value.length === 0) {
    void loadPermissionDefinitions()
  }
}

function openPasswordDialog(user: UserAdminResponse): void {
  closeDialogs()
  passwordUser.value = user
}

function openStatusDialog(user: UserAdminResponse): void {
  closeDialogs()
  statusUser.value = user
  nextStatus.value = user.status === 'active' ? 'disabled' : 'active'
}

function openDeleteDialog(user: UserAdminResponse): void {
  closeDialogs()
  deleteUserTarget.value = user
}

function openActionsDialog(user: UserAdminResponse): void {
  closeDialogs()
  actionsUser.value = user
}

function selectPermissionsAction(): void {
  const target = actionsUser.value
  if (target) {
    openPermissionsDialog(target)
  }
}

function selectPasswordAction(): void {
  const target = actionsUser.value
  if (target) {
    openPasswordDialog(target)
  }
}

function selectStatusAction(): void {
  const target = actionsUser.value
  if (target) {
    openStatusDialog(target)
  }
}

function selectDeleteAction(): void {
  const target = actionsUser.value
  if (target) {
    openDeleteDialog(target)
  }
}

function closeDialogs(): void {
  if (actionSubmitting.value) {
    return
  }
  createDialogOpen.value = false
  permissionsUser.value = null
  passwordUser.value = null
  statusUser.value = null
  deleteUserTarget.value = null
  actionsUser.value = null
  actionError.value = ''
}

/** 使用当前用户的注册权限创建后续账号；成功后回到未筛选第一页展示结果。 */
async function createUser(request: { username: string; password: string }): Promise<void> {
  actionSubmitting.value = true
  actionError.value = ''
  try {
    await registerUser(request)
    createDialogOpen.value = false
    notice.success('用户已创建', {
      detail: `${request.username} 当前默认没有权限。`,
    })
    searchInput.value = ''
    statusInput.value = ''
    activeSearch.value = ''
    activeStatus.value = ''
    page.value = 1
    await loadUsers()
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, '创建用户失败')
    notice.error(actionError.value)
  } finally {
    actionSubmitting.value = false
  }
}

/** 加载权限字典；权限编辑只提交完整代码列表，定义内容不由前端硬编码。 */
async function loadPermissionDefinitions(): Promise<void> {
  permissionsAbortController?.abort()
  const controller = new AbortController()
  permissionsAbortController = controller
  permissionsLoading.value = true
  permissionsLoadError.value = ''
  try {
    permissionDefinitions.value = await listPermissions(controller.signal)
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') {
      return
    }
    permissionsLoadError.value = userManagementErrorMessage(error, '加载权限定义失败')
    notice.error(permissionsLoadError.value)
  } finally {
    if (permissionsAbortController === controller) {
      permissionsAbortController = null
      permissionsLoading.value = false
    }
  }
}

/** 整体替换目标用户权限；修改当前账号时同步前端权限快照。 */
async function savePermissions(permissions: string[]): Promise<void> {
  const target = permissionsUser.value
  if (!target) {
    return
  }
  actionSubmitting.value = true
  actionError.value = ''
  try {
    const updated = await updateUserPermissions(target.id, {
      permissions: [...permissions].sort(),
    })
    replaceUser(updated)
    replaceCurrentSessionPermissions(updated.id, updated.permissions)
    permissionsUser.value = null
    notice.success('权限已更新', {
      detail: `用户 ${updated.username} 的页面导航和操作权限已按新设置生效。`,
    })
    if (isCurrentUser(updated) && !updated.permissions.includes(userPermissions.read)) {
      await router.replace({ name: 'dashboard' })
    }
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, '保存权限失败')
    notice.error(actionError.value)
  } finally {
    actionSubmitting.value = false
  }
}

/** 设置临时密码；服务端会吊销目标用户 refresh token，并要求下次登录改密。 */
async function saveTemporaryPassword(password: string): Promise<void> {
  const target = passwordUser.value
  if (!target) {
    return
  }
  actionSubmitting.value = true
  actionError.value = ''
  try {
    await resetUserPassword(target.id, { password })
    replaceUser({ ...target, password_change_required: true })
    passwordUser.value = null
    notice.success('临时密码已设置', {
      detail: `${target.username} 下次登录后必须修改密码。`,
      durationMs: 6_000,
    })
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, '设置临时密码失败')
    notice.error(actionError.value)
  } finally {
    actionSubmitting.value = false
  }
}

/** 启用或停用目标账号；当前账号不提供自助停用入口。 */
async function saveStatus(): Promise<void> {
  const target = statusUser.value
  if (!target) {
    return
  }
  actionSubmitting.value = true
  actionError.value = ''
  try {
    const updated = await updateUserStatus(target.id, { status: nextStatus.value })
    replaceUser(updated)
    statusUser.value = null
    notice.success(`用户已${updated.status === 'active' ? '启用' : '停用'}`, {
      detail: updated.username,
    })
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, '更新用户状态失败')
    notice.error(actionError.value)
  } finally {
    actionSubmitting.value = false
  }
}

/** 软删除其他账号；成功后刷新当前筛选结果，并避免停留在已经变空的末页。 */
async function confirmDeleteUser(): Promise<void> {
  const target = deleteUserTarget.value
  if (!target) {
    return
  }
  actionSubmitting.value = true
  actionError.value = ''
  try {
    await deleteUser(target.id)
    deleteUserTarget.value = null
    if (users.value.length === 1 && page.value > 1) {
      page.value -= 1
    }
    await loadUsers()
    notice.success('用户已删除', {
      detail: `${target.username} 已退出登录，且无法再次使用该账号。`,
      durationMs: 6_000,
    })
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, '删除用户失败')
    notice.error(actionError.value)
  } finally {
    actionSubmitting.value = false
  }
}

function replaceUser(updated: UserAdminResponse): void {
  users.value = users.value.map((user) => (user.id === updated.id ? updated : user))
}

function isCurrentUser(user: UserAdminResponse): boolean {
  return currentUserId.value === String(user.id)
}

function hasAvailableAction(user: UserAdminResponse): boolean {
  return (
    canEditPermissions.value ||
    (!isCurrentUser(user) &&
      (canResetPassword.value || canUpdateStatus.value || canDelete.value))
  )
}

function statusLabel(status: UserStatus): string {
  return status === 'active' ? '已启用' : '已停用'
}

function statusClass(status: UserStatus): string {
  return status === 'active' ? 'status-pill--ok' : 'status-pill--neutral'
}

function formatDate(value: string): string {
  const date = new Date(value)
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat('zh-CN', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        hour12: false,
      }).format(date)
}

function userManagementErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof ApiError) {
    if (error.code === 'last_permission_manager_required') {
      return '至少需要保留一个已启用且可以管理权限的用户'
    }
    if (error.code === 'username_taken') {
      return '用户名已存在'
    }
    if (error.code === 'permission_denied') {
      return '当前账号没有执行此操作的权限'
    }
    if (error.code === 'user_not_found') {
      return '用户不存在或已被删除'
    }
    if (error.code === 'self_user_delete_forbidden') {
      return '不能删除当前登录账号'
    }
    if (error.code === 'permission_not_found') {
      return '权限定义已变化，请重新加载后再试'
    }
    return Object.keys(error.fieldErrors).length > 0 ? '请检查输入内容' : error.message
  }
  if (error instanceof ApiConfigurationError) {
    return error.message
  }
  if (error instanceof ApiNetworkError) {
    return '无法连接到 WineStock 服务'
  }
  if (error instanceof ApiResponseError) {
    return '服务响应格式无效，请检查前后端版本'
  }
  return fallback
}
</script>

<style lang="scss" src="./UsersPage.scss"></style>
