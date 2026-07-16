<!--
  本文件拥有用户权限分类选择器，属于 frontend 用户管理组件层。
  它维护表单选择和当前账号关键权限锁定，不调用权限 API，也不替代后端授权校验。
-->
<template>
  <ModalDialog
    :open="Boolean(user)"
    title="配置权限"
    :busy="submitting"
    wide
    @close="emit('close')"
  >
    <template #context>
      <div v-if="user" class="permission-picker__context">
        <div>
          <span>目标用户</span>
          <strong :title="user.username">{{ user.username }}</strong>
        </div>
        <div class="permission-picker__total">
          <span>已选权限</span>
          <strong>{{ selectedPermissions.length }} / {{ permissions.length }}</strong>
        </div>
      </div>
    </template>

    <div v-if="loading" class="dialog-state" role="status">正在加载权限…</div>
    <div v-else-if="loadError" class="dialog-state dialog-state--error" role="alert">
      <p>{{ loadError }}</p>
      <button class="secondary-button" type="button" @click="emit('retry')">重试</button>
    </div>
    <form
      v-else
      id="user-permissions-form"
      class="permission-picker"
      novalidate
      @submit.prevent="submitPermissions"
    >
      <nav
        ref="groupsViewport"
        class="permission-picker__groups"
        :class="{ 'permission-picker__groups--scrollable': permissionGroups.length > 4 }"
        aria-label="权限分类"
      >
        <button
          v-for="group in permissionGroups"
          :key="group.name"
          class="permission-picker__group"
          :class="{ 'permission-picker__group--active': activeGroup?.name === group.name }"
          type="button"
          :aria-pressed="activeGroup?.name === group.name"
          @click="activeGroupName = group.name"
        >
          <span>{{ group.name }}</span>
        </button>
      </nav>

      <Transition name="permission-panel" mode="out-in" @after-enter="resetOptionsScroll">
        <section v-if="activeGroup" :key="activeGroup.name" class="permission-picker__panel">
          <header class="permission-picker__panel-header">
            <div>
              <h3>{{ activeGroup.name }}</h3>
              <p>
                已选 {{ selectedCount(activeGroup.permissions) }} 项，共
                {{ activeGroup.permissions.length }} 项
              </p>
            </div>
            <button
              class="secondary-button permission-picker__toggle"
              type="button"
              :disabled="submitting || !hasEditablePermissions(activeGroup.permissions)"
              @click="toggleGroup(activeGroup.permissions)"
            >
              {{ isGroupFullySelected(activeGroup.permissions) ? "取消全选" : "全选" }}
            </button>
          </header>

          <div ref="optionsViewport" class="permission-picker__options">
            <label
              v-for="permission in activeGroup.permissions"
              :key="permission.code"
              class="permission-picker__option"
              :class="{
                'permission-picker__option--locked': isPermissionLocked(permission.code),
              }"
            >
              <input
                v-model="selectedPermissions"
                type="checkbox"
                name="permissions"
                :value="permission.code"
                :disabled="submitting || isPermissionLocked(permission.code)"
              />
              <span>
                <strong>{{ permission.code }}</strong>
                <small v-if="permission.description">{{ permission.description }}</small>
                <small
                  v-if="isPermissionLocked(permission.code)"
                  class="permission-picker__lock-hint"
                >
                  当前账号关键权限，不可修改
                </small>
              </span>
            </label>
          </div>

          <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
        </section>
      </Transition>
    </form>

    <template v-if="editingCurrentUser" #notice>
      <p class="form-warning">
        当前账号的权限管理和权限定义读取能力已锁定；其它权限保存后会立即生效。
      </p>
    </template>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        取消
      </button>
      <button
        class="primary-button"
        type="submit"
        form="user-permissions-form"
        :disabled="submitting || loading || Boolean(loadError)"
      >
        {{ submitting ? "正在保存…" : "保存权限" }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import type { PermissionResponse, UserAdminResponse } from "../../api/users";
import { userPermissions } from "../../auth/permissions";
import ModalDialog from "../ModalDialog.vue";

interface PermissionGroup {
  /** 面向用户展示的权限分类名称。 */
  name: string;
  /** 当前分类拥有的权限定义。 */
  permissions: PermissionResponse[];
}

const props = defineProps<{
  user: UserAdminResponse | null;
  permissions: PermissionResponse[];
  loading: boolean;
  loadError: string;
  submitting: boolean;
  errorMessage: string;
  editingCurrentUser: boolean;
}>();

const emit = defineEmits<{
  close: [];
  retry: [];
  submit: [permissions: string[]];
}>();

const selectedPermissions = ref<string[]>([]);
const activeGroupName = ref("");
const groupsViewport = ref<HTMLElement | null>(null);
const optionsViewport = ref<HTMLElement | null>(null);
const selfProtectedPermissions = new Set<string>([
  userPermissions.updatePermissions,
  userPermissions.readPermissionDefinitions,
]);

const permissionGroups = computed<PermissionGroup[]>(() => {
  const groups = new Map<string, PermissionResponse[]>();
  for (const permission of props.permissions) {
    const name = permission.code.startsWith("user.")
      ? "用户管理"
      : permission.code.startsWith("stock.")
        ? "库存业务"
        : permission.code.startsWith("audit.")
          ? "审计"
          : "其它";
    const entries = groups.get(name) ?? [];
    entries.push(permission);
    groups.set(name, entries);
  }
  return Array.from(groups, ([name, permissions]) => ({ name, permissions }));
});

const activeGroup = computed(
  () =>
    permissionGroups.value.find((group) => group.name === activeGroupName.value) ??
    permissionGroups.value[0],
);

watch(
  () => props.user,
  (user) => {
    selectedPermissions.value = user ? [...user.permissions] : [];
  },
  { immediate: true },
);

watch(
  permissionGroups,
  (groups) => {
    if (!groups.some((group) => group.name === activeGroupName.value)) {
      activeGroupName.value = groups[0]?.name ?? "";
    }
  },
  { immediate: true },
);

watch(
  [activeGroupName, permissionGroups],
  async () => {
    if (permissionGroups.value.length <= 4) return;
    await nextTick();
    groupsViewport.value
      ?.querySelector<HTMLElement>(".permission-picker__group--active")
      ?.scrollIntoView({
        behavior: "smooth",
        block: "nearest",
        inline: "nearest",
      });
  },
  { flush: "post" },
);

function resetOptionsScroll(): void {
  if (optionsViewport.value) {
    optionsViewport.value.scrollTop = 0;
  }
}

function selectedCount(permissions: PermissionResponse[]): number {
  return permissions.filter((permission) => selectedPermissions.value.includes(permission.code))
    .length;
}

/** 当前账号的两项关键权限必须保持打开对话框时的原值。 */
function isPermissionLocked(permissionCode: string): boolean {
  return props.editingCurrentUser && selfProtectedPermissions.has(permissionCode);
}

function editablePermissions(permissions: PermissionResponse[]): PermissionResponse[] {
  return permissions.filter((permission) => !isPermissionLocked(permission.code));
}

function hasEditablePermissions(permissions: PermissionResponse[]): boolean {
  return editablePermissions(permissions).length > 0;
}

function isGroupFullySelected(permissions: PermissionResponse[]): boolean {
  const editable = editablePermissions(permissions);
  return (
    editable.length > 0 &&
    editable.every((permission) => selectedPermissions.value.includes(permission.code))
  );
}

/** 切换当前权限分类的可编辑权限，不改动当前账号的两项关键权限。 */
function toggleGroup(permissions: PermissionResponse[]): void {
  const codes = new Set(editablePermissions(permissions).map((permission) => permission.code));
  if (codes.size === 0) {
    return;
  }
  if (isGroupFullySelected(permissions)) {
    selectedPermissions.value = selectedPermissions.value.filter((code) => !codes.has(code));
    return;
  }
  selectedPermissions.value = Array.from(new Set([...selectedPermissions.value, ...codes]));
}

/** 提交前恢复当前账号关键权限的原值，避免表单状态意外绕过禁用选项。 */
function submitPermissions(): void {
  const permissions = new Set(selectedPermissions.value);
  if (props.editingCurrentUser && props.user) {
    for (const permissionCode of selfProtectedPermissions) {
      if (props.user.permissions.includes(permissionCode)) {
        permissions.add(permissionCode);
      } else {
        permissions.delete(permissionCode);
      }
    }
  }
  emit("submit", Array.from(permissions));
}
</script>

<style lang="scss" src="./UserPermissionsDialog.scss"></style>
