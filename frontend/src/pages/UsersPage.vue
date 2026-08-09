<!--
  本文件拥有用户管理页面的数据加载、筛选、无限滚动和管理操作编排，属于 frontend 页面层。
  它只通过 HTTP API 管理用户，不直接访问 token、数据库或后端内部业务对象。
-->
<template>
  <section class="route-page users-page">
    <header class="content-header users-page__header">
      <div>
        <h1>{{ $title($route.meta.title) }}</h1>
        <p>{{ $t("users.subtitle") }}</p>
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
        <h2>{{ $t("users.loadFailed") }}</h2>
        <p>{{ loadError }}</p>
        <button class="secondary-button" type="button" @click="resetAndLoadUsers">{{ $t("common.retry") }}</button>
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
          {{ $t("users.loadingUsers") }}
        </div>
        <div v-else v-overlay-scrollbar class="users-results">
          <template v-if="users.length > 0">
            <UserDirectoryTable
              :users="users"
              :current-user-id="currentUserId"
              :can-update-username="canUpdateUsername"
              :can-edit-permissions="canEditPermissions"
              :can-reset-password="canResetPassword"
              :can-update-status="canUpdateStatus"
              :can-delete="canDelete"
              @username="openUsernameDialog"
              @permissions="openPermissionsDialog"
              @password="openPasswordDialog"
              @status="openStatusDialog"
              @delete="openDeleteDialog"
            />
            <UserDirectoryMobileList
              :users="users"
              :current-user-id="currentUserId"
              :can-update-username="canUpdateUsername"
              :can-edit-permissions="canEditPermissions"
              :can-reset-password="canResetPassword"
              :can-update-status="canUpdateStatus"
              :can-delete="canDelete"
              @actions="openActionsDialog"
            />
          </template>

          <div ref="loadMoreSentinel" class="users-load-more" aria-live="polite">
            <Transition name="user-count" mode="out-in">
              <span v-if="loadingMore" key="loading" role="status">{{
                $t("users.loadingMoreUsers")
              }}</span>
              <button
                v-else-if="loadMoreError"
                key="error"
                class="secondary-button"
                type="button"
                @click="loadNextPage"
              >
                {{ $t("users.loadMoreFailedRetry") }}
              </button>
              <span v-else-if="hasMoreUsers" key="more">{{ $t("users.scrollToLoadMore") }}</span>
              <span v-else :key="`loaded-${total}`">{{
                $t("users.loadedAllUsers", { n: total })
              }}</span>
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
    <UserUsernameDialog
      :user="usernameUser"
      :submitting="actionSubmitting"
      :error-message="actionError"
      :server-field-errors="actionFieldErrors"
      @close="closeDialogs"
      @submit="saveUsername"
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
      :can-update-username="canUpdateUsername"
      :can-edit-permissions="canEditPermissions"
      :can-reset-password="Boolean(actionsUser && canResetPassword && !isCurrentUser(actionsUser))"
      :can-update-status="Boolean(actionsUser && canUpdateStatus && !isCurrentUser(actionsUser))"
      :can-delete="Boolean(actionsUser && canDelete && !isCurrentUser(actionsUser))"
      @close="closeDialogs"
      @username="selectUsernameAction"
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
import { useI18n } from "vue-i18n";
import { getCurrentUser } from "../api/auth";
import {
  deleteUser,
  listPermissions,
  listUsers,
  registerUser,
  resetUserPassword,
  updateUserUsername,
  updateUserPermissions,
  updateUserStatus,
  type PermissionResponse,
  type UserAdminResponse,
  type UserStatus,
} from "../api/users";
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from "../api/errors";
import { translateMessageOrNull } from "../i18n";
import { hasPermission, userPermissions } from "../auth/permissions";
import {
  authSession,
  replaceCurrentSessionPermissions,
  replaceCurrentSessionUser,
} from "../auth/session";
import UserActionsDialog from "../components/users/UserActionsDialog.vue";
import UserCreateDialog from "../components/users/UserCreateDialog.vue";
import UserDeleteDialog from "../components/users/UserDeleteDialog.vue";
import UserDirectoryMobileList from "../components/users/UserDirectoryMobileList.vue";
import UserDirectoryTable from "../components/users/UserDirectoryTable.vue";
import UserListToolbar from "../components/users/UserListToolbar.vue";
import UserPasswordResetDialog from "../components/users/UserPasswordResetDialog.vue";
import UserPermissionsDialog from "../components/users/UserPermissionsDialog.vue";
import UserStatusDialog from "../components/users/UserStatusDialog.vue";
import UserUsernameDialog from "../components/users/UserUsernameDialog.vue";
import { useStablePendingIndicator } from "../composables/useStablePendingIndicator";
import { notice } from "../notices/notice";

const PAGE_SIZE = 20;

const router = useRouter();
const { t } = useI18n();
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
const usernameUser = ref<UserAdminResponse | null>(null);
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
const canUpdateUsername = computed(() =>
  hasPermission(currentPermissions.value, userPermissions.updateUsername),
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
      shouldAppend ? t("users.loadMoreUsersFailed") : t("users.loadUsersFailed"),
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
    notice.success(t("users.listRefreshed"));
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

function openUsernameDialog(user: UserAdminResponse): void {
  closeDialogs();
  usernameUser.value = user;
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

function selectUsernameAction(): void {
  const target = actionsUser.value;
  if (target) {
    openUsernameDialog(target);
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
  usernameUser.value = null;
  passwordUser.value = null;
  statusUser.value = null;
  deleteUserTarget.value = null;
  actionsUser.value = null;
  actionError.value = "";
  actionFieldErrors.value = {};
}

/** 修改登录用户名；用户 ID、权限、密码和现有会话不因用户名变化而改变。 */
async function saveUsername(username: string): Promise<void> {
  const target = usernameUser.value;
  if (!target) {
    return;
  }
  actionSubmitting.value = true;
  actionError.value = "";
  actionFieldErrors.value = {};
  try {
    const updated = await updateUserUsername(target.id, { username });
    replaceUser(updated);
    if (isCurrentUser(updated)) {
      replaceCurrentSessionUser(await getCurrentUser());
    }
    usernameUser.value = null;
    notice.success(t("users.usernameUpdated"), {
      detail: t("users.usernameUpdatedDetail", { username: updated.username }),
    });
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, t("users.updateUsernameFailed"));
    actionFieldErrors.value = userManagementFieldErrors(error);
    notice.error(actionError.value, { detail: Object.values(actionFieldErrors.value)[0] });
  } finally {
    actionSubmitting.value = false;
  }
}

/** 使用当前用户的注册权限创建后续账号；成功后回到未筛选第一页展示结果。 */
async function createUser(request: { username: string; password: string }): Promise<void> {
  actionSubmitting.value = true;
  actionError.value = "";
  actionFieldErrors.value = {};
  try {
    await registerUser(request);
    createDialogOpen.value = false;
    notice.success(t("users.userCreated"), {
      detail: t("users.userCreatedDetail", { username: request.username }),
    });
    searchInput.value = "";
    statusInput.value = "";
    activeSearch.value = "";
    activeStatus.value = "";
    await resetAndLoadUsers();
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, t("users.createFailed"));
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
    permissionsLoadError.value = userManagementErrorMessage(
      error,
      t("users.loadPermissionDefinitionsFailed"),
    );
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
    notice.success(t("users.permissionsUpdated"), {
      detail: t("users.permissionsUpdatedDetail", { username: updated.username }),
    });
    if (isCurrentUser(updated) && !updated.permissions.includes(userPermissions.read)) {
      await router.replace({ name: "dashboard" });
    }
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, t("users.savePermissionsFailed"));
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
    notice.success(t("users.temporaryPasswordSet"), {
      detail: t("users.temporaryPasswordSetDetail", { username: target.username }),
    });
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, t("users.setTemporaryPasswordFailed"));
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
    notice.success(
      updated.status === "active" ? t("users.enabledNotice") : t("users.disabledNotice"),
      { detail: updated.username },
    );
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, t("users.updateStatusFailed"));
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
    notice.success(t("users.userDeleted"), {
      detail: t("users.userDeletedDetail", { username: target.username }),
    });
  } catch (error) {
    actionError.value = userManagementErrorMessage(error, t("users.deleteFailed"));
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

function userManagementErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof ApiError) {
    return Object.keys(error.fieldErrors).length > 0
      ? t("users.checkInput")
      : (translateMessageOrNull(`error.${error.code}`) ?? error.message);
  }
  if (error instanceof ApiConfigurationError) {
    return error.message;
  }
  if (error instanceof ApiNetworkError) {
    return t("error.network_unavailable");
  }
  if (error instanceof ApiResponseError) {
    return t("users.responseInvalidFormat");
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
  if (error.code === "username_taken") mapped.username ??= t("error.username_taken");
  return mapped;
}
</script>

<style lang="scss" src="./UsersPage.scss"></style>
