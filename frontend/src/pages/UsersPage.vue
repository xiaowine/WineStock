<!--
  本文件拥有用户管理页面的数据加载、筛选、无限滚动和管理操作编排，属于 frontend 页面层。
  它只通过 HTTP API 管理用户，不直接访问 token、数据库或后端内部业务对象。
-->
<template>
  <section class="route-page users-page">
    <header class="content-header users-page__header">
      <div>
        <h1>{{ $route.meta.title }}</h1>
        <p>查看账号状态，并按权限执行创建、启停、删除、权限分配和临时密码操作。</p>
      </div>
    </header>

    <div class="users-page__workspace">
      <UserListToolbar
        v-model:search="searchInput"
        v-model:status="statusInput"
        :total="total"
        :loading="listRequestPending"
        :refreshing="showStableUserRefreshing"
        :can-register="canRegister"
        @apply="applyFilters"
        @search="applySearch"
        @refresh="refreshUsers"
        @create="openCreateDialog"
      />

      <section v-if="loadError" class="page-state page-state--error" role="alert">
        <h2>无法加载用户</h2>
        <p>{{ loadError }}</p>
        <button class="secondary-button" type="button" @click="resetAndLoadUsers">重试</button>
      </section>

      <section
        v-else
        class="users-content"
        :class="{ 'users-content--refreshing': showStableUserRefreshing }"
        :aria-busy="loading"
      >
        <div
          v-if="loading && users.length === 0 && !hasLoadedUserList"
          class="page-state"
          role="status"
        >
          正在加载用户…
        </div>

        <div v-else class="users-results">
          <div v-if="users.length > 0" class="users-table-wrap">
            <div class="users-table" role="table" aria-label="用户列表">
              <div class="users-table__header" role="row">
                <span class="users-table__fixed-start" role="columnheader">用户</span>
                <div class="users-table__middle" role="group">
                  <span role="columnheader">状态</span>
                  <span role="columnheader">权限</span>
                  <span role="columnheader">密码</span>
                  <span role="columnheader">最近更新</span>
                </div>
                <span class="users-table__fixed-end" role="columnheader">操作</span>
              </div>

              <article v-for="user in users" :key="user.id" class="users-table__row" role="row">
                <div class="users-table__fixed-start" role="cell">
                  <span class="user-identity-cell">
                    <strong :title="user.username">{{ user.username }}</strong>
                    <small
                      >#{{ user.id
                      }}<template v-if="isCurrentUser(user)"> · 当前账号</template></small
                    >
                  </span>
                </div>

                <div class="users-table__middle" role="group">
                  <div role="cell">
                    <span class="status-pill" :class="statusClass(user.status)">
                      {{ statusLabel(user.status) }}
                    </span>
                  </div>
                  <div role="cell">{{ user.permissions.length }} 项</div>
                  <div role="cell">
                    <span
                      class="status-pill"
                      :class="
                        user.password_change_required ? 'status-pill--warn' : 'status-pill--neutral'
                      "
                    >
                      {{ user.password_change_required ? "待修改" : "正常" }}
                    </span>
                  </div>
                  <div role="cell">{{ formatDate(user.updated_at) }}</div>
                </div>

                <div class="users-table__fixed-end" role="cell">
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
                      {{ user.status === "active" ? "停用" : "启用" }}
                    </button>
                    <button
                      v-if="canDelete && !isCurrentUser(user)"
                      class="text-button"
                      type="button"
                      @click="openDeleteDialog(user)"
                    >
                      删除
                    </button>
                    <span v-if="!hasAvailableAction(user)" class="row-actions__empty"
                      >无可用操作</span
                    >
                  </div>
                </div>
              </article>
            </div>
          </div>

          <div v-if="users.length > 0" class="users-mobile-list">
            <article v-for="user in users" :key="user.id" class="user-mobile-item">
              <header>
                <div>
                  <h2 :title="user.username">{{ user.username }}</h2>
                  <p>#{{ user.id }}<template v-if="isCurrentUser(user)"> · 当前账号</template></p>
                </div>
                <div class="user-mobile-item__actions">
                  <span class="status-pill" :class="statusClass(user.status)">
                    {{ statusLabel(user.status) }}
                  </span>
                  <button
                    v-if="hasAvailableAction(user)"
                    class="icon-button user-mobile-item__edit"
                    type="button"
                    title="管理用户"
                    :aria-label="`管理用户：${user.username}`"
                    @click="openActionsDialog(user)"
                  >
                    <svg viewBox="0 0 24 24" aria-hidden="true">
                      <path d="M4 20h4l11-11-4-4L4 16v4Z" />
                      <path d="m13.5 6.5 4 4" />
                    </svg>
                  </button>
                </div>
              </header>
              <dl>
                <div>
                  <dt>权限</dt>
                  <dd>{{ user.permissions.length }} 项</dd>
                </div>
                <div>
                  <dt>密码状态</dt>
                  <dd>{{ user.password_change_required ? "待修改临时密码" : "正常" }}</dd>
                </div>
                <div>
                  <dt>最近更新</dt>
                  <dd>{{ formatDate(user.updated_at) }}</dd>
                </div>
              </dl>
            </article>
          </div>

          <div ref="loadMoreSentinel" class="users-load-more" aria-live="polite">
            <Transition name="user-count" mode="out-in">
              <span v-if="loadingMore" key="loading" role="status">正在加载更多用户…</span>
              <button
                v-else-if="loadMoreError"
                key="error"
                class="secondary-button"
                type="button"
                @click="loadNextPage"
              >
                加载失败，点击重试
              </button>
              <span v-else-if="hasMoreUsers" key="more">继续向下滚动加载</span>
              <span v-else :key="`loaded-${total}`">已加载全部 {{ total }} 个用户</span>
            </Transition>
          </div>
        </div>
      </section>
    </div>

    <UserCreateDialog
      :open="createDialogOpen"
      :submitting="actionSubmitting"
      :error-message="actionError"
      :server-field-errors="actionFieldErrors"
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
      :server-field-errors="actionFieldErrors"
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
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
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
} from "../api/users";
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from "../api/errors";
import { hasPermission, userPermissions } from "../auth/permissions";
import { authSession, replaceCurrentSessionPermissions } from "../auth/session";
import UserActionsDialog from "../components/users/UserActionsDialog.vue";
import UserCreateDialog from "../components/users/UserCreateDialog.vue";
import UserDeleteDialog from "../components/users/UserDeleteDialog.vue";
import UserListToolbar from "../components/users/UserListToolbar.vue";
import UserPasswordResetDialog from "../components/users/UserPasswordResetDialog.vue";
import UserPermissionsDialog from "../components/users/UserPermissionsDialog.vue";
import UserStatusDialog from "../components/users/UserStatusDialog.vue";
import { useStablePendingIndicator } from "../composables/useStablePendingIndicator";
import { notice } from "../notices/notice";

const PAGE_SIZE = 20;

const router = useRouter();
const users = ref<UserAdminResponse[]>([]);
const total = ref(0);
const page = ref(1);
const totalPages = ref(0);
const searchInput = ref("");
const statusInput = ref<"" | UserStatus>("");
const activeSearch = ref("");
const activeStatus = ref<"" | UserStatus>("");
const hasLoadedUserList = ref(false);
const loading = ref(false);
const loadingMore = ref(false);
const loadError = ref("");
const loadMoreError = ref("");
const loadMoreSentinel = ref<HTMLElement | null>(null);
const createDialogOpen = ref(false);
const permissionsUser = ref<UserAdminResponse | null>(null);
const passwordUser = ref<UserAdminResponse | null>(null);
const statusUser = ref<UserAdminResponse | null>(null);
const deleteUserTarget = ref<UserAdminResponse | null>(null);
const actionsUser = ref<UserAdminResponse | null>(null);
const nextStatus = ref<UserStatus>("disabled");
const actionSubmitting = ref(false);
const actionError = ref("");
const actionFieldErrors = ref<Record<string, string>>({});
const permissionDefinitions = ref<PermissionResponse[]>([]);
const permissionsLoading = ref(false);
const permissionsLoadError = ref("");
let usersAbortController: AbortController | null = null;
let permissionsAbortController: AbortController | null = null;
let loadMoreObserver: IntersectionObserver | null = null;

const currentPermissions = computed(() => authSession.value?.user.permissions);
const currentUserId = computed(() => authSession.value?.user.id);
const canRegister = computed(() =>
  hasPermission(currentPermissions.value, userPermissions.register),
);
const canUpdateStatus = computed(() =>
  hasPermission(currentPermissions.value, userPermissions.updateStatus),
);
const canDelete = computed(() => hasPermission(currentPermissions.value, userPermissions.delete));
const canResetPassword = computed(() =>
  hasPermission(currentPermissions.value, userPermissions.resetPassword),
);
const canEditPermissions = computed(
  () =>
    hasPermission(currentPermissions.value, userPermissions.updatePermissions) &&
    hasPermission(currentPermissions.value, userPermissions.readPermissionDefinitions),
);
const listRequestPending = computed(() => loading.value || loadingMore.value);
const hasMoreUsers = computed(() => page.value < totalPages.value);
const userRefreshPending = computed(() => hasLoadedUserList.value && loading.value);
const showStableUserRefreshing = useStablePendingIndicator(userRefreshPending, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});

watch(loadMoreSentinel, (element, previousElement) => {
  if (previousElement) {
    loadMoreObserver?.unobserve(previousElement);
  }
  if (element) {
    loadMoreObserver?.observe(element);
  }
});

onMounted(() => {
  loadMoreObserver = new IntersectionObserver(handleLoadMoreIntersection, {
    rootMargin: "240px 0px",
  });
  if (loadMoreSentinel.value) {
    loadMoreObserver.observe(loadMoreSentinel.value);
  }
  void loadUsers(1);
});
onBeforeUnmount(() => {
  usersAbortController?.abort();
  permissionsAbortController?.abort();
  loadMoreObserver?.disconnect();
});

/** 查询指定用户页；追加模式保留已加载数据，新请求会取消旧请求以避免响应乱序。 */
async function loadUsers(targetPage: number, append = false): Promise<boolean> {
  usersAbortController?.abort();
  const controller = new AbortController();
  usersAbortController = controller;
  const shouldAppend = append && users.value.length > 0;
  loading.value = !shouldAppend;
  loadingMore.value = shouldAppend;
  loadMoreError.value = "";
  if (!shouldAppend) {
    loadError.value = "";
  }
  let requestSucceeded = false;

  try {
    const response = await listUsers(
      {
        page: targetPage,
        page_size: PAGE_SIZE,
        search: activeSearch.value || undefined,
        status: activeStatus.value || undefined,
      },
      controller.signal,
    );
    users.value = shouldAppend ? mergeUsers(users.value, response.items) : response.items;
    total.value = response.total;
    totalPages.value = response.total_pages;
    page.value = response.page;
    hasLoadedUserList.value = true;
    requestSucceeded = true;
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") {
      return false;
    }
    const message = userManagementErrorMessage(
      error,
      shouldAppend ? "加载更多用户失败" : "加载用户失败",
    );
    if (shouldAppend) {
      loadMoreError.value = message;
    } else if (!hasLoadedUserList.value) {
      loadError.value = message;
    }
    notice.error(message);
  } finally {
    if (usersAbortController === controller) {
      usersAbortController = null;
      loading.value = false;
      loadingMore.value = false;
      if (requestSucceeded) {
        void nextTick().then(refreshLoadMoreObservation);
      }
    }
  }
  return requestSucceeded;
}

/** 从第一页重新加载；请求期间保留现有列表，成功后替换内容并重建分页状态。 */
async function resetAndLoadUsers(): Promise<boolean> {
  loadMoreError.value = "";
  return loadUsers(1);
}

/** 手动刷新用户列表；刷新后从第一页重新开始无限滚动。 */
async function refreshUsers(): Promise<void> {
  const refreshed = await resetAndLoadUsers();
  if (refreshed) {
    notice.success("用户列表已刷新");
  }
}

function applyFilters(): void {
  activeStatus.value = statusInput.value;
  void resetAndLoadUsers();
}

function applySearch(value: string): void {
  if (value === activeSearch.value) return;
  activeSearch.value = value;
  void resetAndLoadUsers();
}

/** 哨兵进入视口预加载范围时请求下一页；同一时刻只允许一个列表请求。 */
function handleLoadMoreIntersection(entries: IntersectionObserverEntry[]): void {
  if (entries.some((entry) => entry.isIntersecting)) {
    void loadNextPage();
  }
}

async function loadNextPage(): Promise<void> {
  if (listRequestPending.value || !hasMoreUsers.value) {
    return;
  }
  await loadUsers(page.value + 1, true);
}

/** 追加后重新观察哨兵，使短列表可以继续加载直到填满可视区域。 */
function refreshLoadMoreObservation(): void {
  const sentinel = loadMoreSentinel.value;
  if (!sentinel || !loadMoreObserver) {
    return;
  }
  loadMoreObserver.unobserve(sentinel);
  loadMoreObserver.observe(sentinel);
}

function openCreateDialog(): void {
  closeDialogs();
  createDialogOpen.value = true;
}

function openPermissionsDialog(user: UserAdminResponse): void {
  closeDialogs();
  permissionsUser.value = user;
  if (permissionDefinitions.value.length === 0) {
    void loadPermissionDefinitions();
  }
}

function openPasswordDialog(user: UserAdminResponse): void {
  closeDialogs();
  passwordUser.value = user;
}

function openStatusDialog(user: UserAdminResponse): void {
  closeDialogs();
  statusUser.value = user;
  nextStatus.value = user.status === "active" ? "disabled" : "active";
}

function openDeleteDialog(user: UserAdminResponse): void {
  closeDialogs();
  deleteUserTarget.value = user;
}

function openActionsDialog(user: UserAdminResponse): void {
  closeDialogs();
  actionsUser.value = user;
}

function selectPermissionsAction(): void {
  const target = actionsUser.value;
  if (target) {
    openPermissionsDialog(target);
  }
}

function selectPasswordAction(): void {
  const target = actionsUser.value;
  if (target) {
    openPasswordDialog(target);
  }
}

function selectStatusAction(): void {
  const target = actionsUser.value;
  if (target) {
    openStatusDialog(target);
  }
}

function selectDeleteAction(): void {
  const target = actionsUser.value;
  if (target) {
    openDeleteDialog(target);
  }
}

function closeDialogs(): void {
  if (actionSubmitting.value) {
    return;
  }
  createDialogOpen.value = false;
  permissionsUser.value = null;
  passwordUser.value = null;
  statusUser.value = null;
  deleteUserTarget.value = null;
  actionsUser.value = null;
  actionError.value = "";
  actionFieldErrors.value = {};
}

/** 使用当前用户的注册权限创建后续账号；成功后回到未筛选第一页展示结果。 */
async function createUser(request: { username: string; password: string }): Promise<void> {
  actionSubmitting.value = true;
  actionError.value = "";
  actionFieldErrors.value = {};
  try {
    await registerUser(request);
    createDialogOpen.value = false;
    notice.success("用户已创建", {
      detail: `${request.username} 当前默认没有权限。`,
    });
    searchInput.value = "";
    statusInput.value = "";
    activeSearch.value = "";
    activeStatus.value = "";
    await resetAndLoadUsers();
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, "创建用户失败");
    actionFieldErrors.value = userManagementFieldErrors(error);
    notice.error(actionError.value, { detail: Object.values(actionFieldErrors.value)[0] });
  } finally {
    actionSubmitting.value = false;
  }
}

/** 加载权限字典；权限编辑只提交完整代码列表，定义内容不由前端硬编码。 */
async function loadPermissionDefinitions(): Promise<void> {
  permissionsAbortController?.abort();
  const controller = new AbortController();
  permissionsAbortController = controller;
  permissionsLoading.value = true;
  permissionsLoadError.value = "";
  try {
    permissionDefinitions.value = await listPermissions(controller.signal);
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") {
      return;
    }
    permissionsLoadError.value = userManagementErrorMessage(error, "加载权限定义失败");
    notice.error(permissionsLoadError.value, { onClick: () => void loadPermissionDefinitions() });
  } finally {
    if (permissionsAbortController === controller) {
      permissionsAbortController = null;
      permissionsLoading.value = false;
    }
  }
}

/** 整体替换目标用户权限；修改当前账号时同步前端权限快照。 */
async function savePermissions(permissions: string[]): Promise<void> {
  const target = permissionsUser.value;
  if (!target) {
    return;
  }
  actionSubmitting.value = true;
  actionError.value = "";
  try {
    const updated = await updateUserPermissions(target.id, {
      permissions: [...permissions].sort(),
    });
    replaceUser(updated);
    replaceCurrentSessionPermissions(updated.id, updated.permissions);
    permissionsUser.value = null;
    notice.success("权限已更新", {
      detail: `用户 ${updated.username} 的页面导航和操作权限已按新设置生效。`,
    });
    if (isCurrentUser(updated) && !updated.permissions.includes(userPermissions.read)) {
      await router.replace({ name: "dashboard" });
    }
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, "保存权限失败");
    notice.error(actionError.value);
  } finally {
    actionSubmitting.value = false;
  }
}

/** 设置临时密码；服务端会吊销目标用户 refresh token，并要求下次登录改密。 */
async function saveTemporaryPassword(password: string): Promise<void> {
  const target = passwordUser.value;
  if (!target) {
    return;
  }
  actionSubmitting.value = true;
  actionError.value = "";
  actionFieldErrors.value = {};
  try {
    await resetUserPassword(target.id, { password });
    replaceUser({ ...target, password_change_required: true });
    passwordUser.value = null;
    notice.success("临时密码已设置", {
      detail: `${target.username} 下次登录后必须修改密码。`,
    });
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, "设置临时密码失败");
    actionFieldErrors.value = userManagementFieldErrors(error);
    notice.error(actionError.value, { detail: Object.values(actionFieldErrors.value)[0] });
  } finally {
    actionSubmitting.value = false;
  }
}

/** 启用或停用目标账号；当前账号不提供自助停用入口。 */
async function saveStatus(): Promise<void> {
  const target = statusUser.value;
  if (!target) {
    return;
  }
  actionSubmitting.value = true;
  actionError.value = "";
  try {
    const updated = await updateUserStatus(target.id, { status: nextStatus.value });
    statusUser.value = null;
    await resetAndLoadUsers();
    notice.success(`用户已${updated.status === "active" ? "启用" : "停用"}`, {
      detail: updated.username,
    });
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, "更新用户状态失败");
    notice.error(actionError.value);
  } finally {
    actionSubmitting.value = false;
  }
}

/** 软删除其他账号；成功后从第一页重建无限列表，避免服务端分页前移造成漏项。 */
async function confirmDeleteUser(): Promise<void> {
  const target = deleteUserTarget.value;
  if (!target) {
    return;
  }
  actionSubmitting.value = true;
  actionError.value = "";
  try {
    await deleteUser(target.id);
    deleteUserTarget.value = null;
    await resetAndLoadUsers();
    notice.success("用户已删除", {
      detail: `${target.username} 已退出登录，且无法再次使用该账号。`,
    });
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, "删除用户失败");
    notice.error(actionError.value);
  } finally {
    actionSubmitting.value = false;
  }
}

function replaceUser(updated: UserAdminResponse): void {
  users.value = users.value.map((user) => (user.id === updated.id ? updated : user));
}

function mergeUsers(
  currentUsers: UserAdminResponse[],
  nextUsers: UserAdminResponse[],
): UserAdminResponse[] {
  const usersById = new Map(currentUsers.map((user) => [user.id, user]));
  nextUsers.forEach((user) => usersById.set(user.id, user));
  return Array.from(usersById.values());
}

function isCurrentUser(user: UserAdminResponse): boolean {
  return currentUserId.value === String(user.id);
}

function hasAvailableAction(user: UserAdminResponse): boolean {
  return (
    canEditPermissions.value ||
    (!isCurrentUser(user) && (canResetPassword.value || canUpdateStatus.value || canDelete.value))
  );
}

function statusLabel(status: UserStatus): string {
  return status === "active" ? "已启用" : "已停用";
}

function statusClass(status: UserStatus): string {
  return status === "active" ? "status-pill--ok" : "status-pill--neutral";
}

function formatDate(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat("zh-CN", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      }).format(date);
}

function userManagementErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof ApiError) {
    if (error.code === "last_permission_manager_required") {
      return "至少需要保留一个已启用且可以管理权限的用户";
    }
    if (error.code === "username_taken") {
      return "用户名已存在";
    }
    if (error.code === "permission_denied") {
      return "当前账号没有执行此操作的权限";
    }
    if (error.code === "user_not_found") {
      return "用户不存在或已被删除";
    }
    if (error.code === "self_user_delete_forbidden") {
      return "不能删除当前登录账号";
    }
    if (error.code === "permission_not_found") {
      return "权限定义已变化，请重新加载后再试";
    }
    return Object.keys(error.fieldErrors).length > 0 ? "请检查输入内容" : error.message;
  }
  if (error instanceof ApiConfigurationError) {
    return error.message;
  }
  if (error instanceof ApiNetworkError) {
    return "无法连接到 WineStock 服务";
  }
  if (error instanceof ApiResponseError) {
    return "服务响应格式无效，请检查前后端版本";
  }
  return fallback;
}

function userManagementFieldErrors(error: unknown): Record<string, string> {
  if (!(error instanceof ApiError)) return {};
  const mapped: Record<string, string> = {};
  for (const [path, messages] of Object.entries(error.fieldErrors)) {
    const key = path.replace(/^data\./, "");
    const field =
      key === "password_confirmation" || key === "temporary_password_confirmation"
        ? "confirmation"
        : key === "password" || key === "temporary_password"
          ? "password"
          : key === "username"
            ? "username"
            : "";
    if (field && messages[0]) mapped[field] = messages[0];
  }
  if (error.code === "username_taken") mapped.username ??= "用户名已存在";
  return mapped;
}
</script>

<style lang="scss" src="./UsersPage.scss"></style>
