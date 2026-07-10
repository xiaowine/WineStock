<!--
  本文件拥有全局 Notice 的角落布局、标题详情、点击入口、倒计时展示和悬浮暂停交互。
  它属于 frontend 通用组件层，不生成业务提示内容。
-->
<template>
  <Teleport to="body">
    <TransitionGroup
      tag="section"
      name="notice-list"
      class="notice-viewport"
      aria-label="通知"
      aria-live="polite"
    >
      <article
        v-for="item in notices"
        :key="item.id"
        class="notice-toast"
        :class="`notice-toast--${item.tone}`"
        :role="item.tone === 'error' ? 'alert' : 'status'"
        @pointerenter="pauseNotice(item.id)"
        @pointerleave="resumeNotice(item.id)"
        @focusin="pauseNotice(item.id)"
        @focusout="handleFocusOut(item.id, $event)"
      >
        <div class="notice-toast__content">
          <span class="notice-toast__tone">{{ toneLabel(item.tone) }}</span>
          <component
            :is="item.onClick ? 'button' : 'div'"
            class="notice-toast__copy"
            :class="{ 'notice-toast__action': Boolean(item.onClick) }"
            :type="item.onClick ? 'button' : undefined"
            @click="activateNotice(item.id)"
          >
            <strong>{{ item.title }}</strong>
            <p v-if="item.detail">{{ item.detail }}</p>
          </component>
          <button
            class="notice-toast__close"
            type="button"
            title="关闭通知"
            aria-label="关闭通知"
            @click.stop="dismissNotice(item.id)"
            @keydown.stop
          >
            ×
          </button>
        </div>
        <span class="notice-toast__progress" aria-hidden="true">
          <span :style="{ transform: `scaleX(${progress(item)})` }" />
        </span>
      </article>
    </TransitionGroup>
  </Teleport>
</template>

<script setup lang="ts">
import {
  activateNotice,
  dismissNotice,
  notices,
  pauseNotice,
  resumeNotice,
  type NoticeItem,
  type NoticeTone,
} from '../notices/notice'

function progress(item: NoticeItem): number {
  return Math.max(0, Math.min(1, item.remainingMs / item.durationMs))
}

function toneLabel(tone: NoticeTone): string {
  if (tone === 'success') {
    return '成功'
  }
  if (tone === 'warning') {
    return '警告'
  }
  if (tone === 'error') {
    return '错误'
  }
  return '提示'
}

function handleFocusOut(id: string, event: FocusEvent): void {
  const currentTarget = event.currentTarget
  if (
    currentTarget instanceof HTMLElement &&
    event.relatedTarget instanceof Node &&
    currentTarget.contains(event.relatedTarget)
  ) {
    return
  }
  resumeNotice(id)
}
</script>

<style scoped>
.notice-viewport {
  position: fixed;
  top: calc(10px + env(safe-area-inset-top));
  right: 14px;
  z-index: var(--z-notice);
  display: grid;
  width: min(380px, calc(100vw - 28px));
  gap: 10px;
  pointer-events: none;
}

.notice-toast {
  position: relative;
  overflow: hidden;
  border: 1px solid var(--color-border-strong);
  border-left: 7px solid var(--color-accent);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  box-shadow: var(--shadow-menu);
  color: var(--color-text);
  pointer-events: auto;
}

.notice-toast--success {
  border-left-color: var(--color-teal);
}

.notice-toast--warning {
  border-left-color: var(--color-warn);
}

.notice-toast--error {
  border-left-color: var(--color-danger);
}

.notice-toast__content {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) 30px;
  align-items: start;
  gap: 10px;
  min-height: 58px;
  padding: 12px 10px 13px 12px;
}

.notice-toast__tone {
  display: inline-flex;
  min-height: 24px;
  align-items: center;
  padding: 0 7px;
  border-radius: var(--radius-sm);
  background: var(--color-surface-raised);
  color: var(--color-muted);
  font-size: 12px;
  font-weight: 680;
}

.notice-toast--success .notice-toast__tone {
  background: var(--color-teal-soft);
  color: var(--color-teal);
}

.notice-toast--warning .notice-toast__tone {
  background: var(--color-warn-soft);
  color: var(--color-warn);
}

.notice-toast--error .notice-toast__tone {
  background: var(--color-danger-soft);
  color: var(--color-danger);
}

.notice-toast__copy {
  display: grid;
  min-width: 0;
  gap: 3px;
  padding-top: 2px;
  color: inherit;
  text-align: left;
}

.notice-toast__action {
  width: 100%;
  padding: 2px 4px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  cursor: pointer;
}

.notice-toast__action:hover {
  background: var(--color-surface-raised);
}

.notice-toast__action:focus-visible {
  outline: 3px solid rgb(111 42 54 / 18%);
  outline-offset: 1px;
}

.notice-toast__copy strong,
.notice-toast__copy p {
  overflow-wrap: anywhere;
}

.notice-toast__copy strong {
  font-size: 13px;
  line-height: 1.5;
}

.notice-toast__copy p {
  color: var(--color-muted);
  font-size: 12px;
  line-height: 1.5;
}

.notice-toast__close {
  display: inline-grid;
  width: 30px;
  height: 30px;
  place-items: center;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-muted);
  font-size: 18px;
}

.notice-toast__close:hover {
  background: var(--color-surface-raised);
  color: var(--color-text);
}

.notice-toast__progress {
  position: absolute;
  right: 0;
  bottom: 0;
  left: 0;
  height: 3px;
  background: var(--color-surface-raised);
}

.notice-toast__progress > span {
  display: block;
  width: 100%;
  height: 100%;
  transform-origin: left center;
  background: var(--color-accent);
}

.notice-toast--success .notice-toast__progress > span {
  background: var(--color-teal);
}

.notice-toast--warning .notice-toast__progress > span {
  background: var(--color-warn);
}

.notice-toast--error .notice-toast__progress > span {
  background: var(--color-danger);
}

.notice-list-enter-active,
.notice-list-leave-active {
  transition:
    opacity 160ms ease,
    transform 160ms ease;
}

.notice-list-enter-from,
.notice-list-leave-to {
  opacity: 0;
  transform: translateX(18px);
}

.notice-list-move {
  transition: transform 160ms ease;
}

@media (max-width: 767px) {
  .notice-viewport {
    right: 10px;
    width: calc(100vw - 20px);
  }
}
</style>
