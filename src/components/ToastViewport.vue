<script setup lang="ts">
import { useI18n } from "vue-i18n";
import {
  dismissToast,
  pauseToast,
  resumeToast,
  toasts,
  type ToastItem,
} from "../toast";

const { t } = useI18n();

function runAction(item: ToastItem) {
  item.onAction?.();
  dismissToast(item.id);
}
</script>

<template>
  <div class="toast-viewport" :aria-label="t('notification.regionLabel')">
    <TransitionGroup name="toast-stack">
      <article
        v-for="item in toasts"
        :key="item.id"
        class="app-toast"
        :class="item.intent"
        :role="item.intent === 'error' ? 'alert' : 'status'"
        :aria-live="item.intent === 'error' ? 'assertive' : 'polite'"
        aria-atomic="true"
        @mouseenter="pauseToast(item.id)"
        @mouseleave="resumeToast(item.id)"
        @focusin="pauseToast(item.id)"
        @focusout="resumeToast(item.id)"
      >
        <span v-if="item.intent === 'progress'" class="toast-spinner"></span>
        <svg v-else class="toast-icon" viewBox="0 0 20 20" aria-hidden="true">
          <path
            v-if="item.intent === 'success'"
            d="m5 10.2 3.1 3.1L15.3 6"
          />
          <path
            v-else-if="item.intent === 'warning'"
            d="M10 3.2 17 16H3L10 3.2Zm0 4.2v4.1m0 2.1v.2"
          />
          <path
            v-else-if="item.intent === 'error'"
            d="m6 6 8 8m0-8-8 8"
          />
          <path v-else d="M10 8.1v6m0-9.1v.2" />
        </svg>

        <div class="toast-copy">
          <strong>{{ item.title }}</strong>
          <p v-if="item.message">{{ item.message }}</p>
          <button
            v-if="item.actionLabel"
            type="button"
            class="toast-action"
            @click="runAction(item)"
          >
            {{ item.actionLabel }}
          </button>
        </div>

        <button
          type="button"
          class="toast-close"
          :aria-label="t('notification.close')"
          @click="dismissToast(item.id)"
        >
          ×
        </button>
      </article>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-viewport {
  position: fixed;
  z-index: 900;
  right: 20px;
  bottom: 20px;
  display: flex;
  width: min(350px, calc(100vw - 40px));
  flex-direction: column;
  gap: 10px;
  pointer-events: none;
}

.app-toast {
  --toast-tone: var(--color-accent);
  position: relative;
  display: grid;
  grid-template-columns: 24px minmax(0, 1fr) 24px;
  min-height: 58px;
  gap: 10px;
  align-items: start;
  padding: 12px 10px 12px 12px;
  border: 1px solid color-mix(in srgb, var(--toast-tone) 58%, var(--color-border));
  background: color-mix(in srgb, var(--color-panel) 92%, transparent);
  box-shadow:
    inset 3px 0 var(--toast-tone),
    0 14px 36px color-mix(in srgb, #000 24%, transparent);
  backdrop-filter: blur(16px) saturate(130%);
  pointer-events: auto;
}

.app-toast.success { --toast-tone: var(--color-success-text); }
.app-toast.warning { --toast-tone: var(--color-warning-text); }
.app-toast.error { --toast-tone: var(--color-danger-text); }
.app-toast.progress { --toast-tone: var(--color-accent); }

.toast-icon {
  width: 20px;
  height: 20px;
  margin: 1px 0 0 1px;
  fill: none;
  stroke: var(--toast-tone);
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.8;
}

.toast-spinner {
  box-sizing: border-box;
  width: 18px;
  height: 18px;
  margin: 2px;
  border: 2px solid color-mix(in srgb, var(--toast-tone) 28%, transparent);
  border-top-color: var(--toast-tone);
  border-radius: 50%;
  animation: toast-spin 700ms linear infinite;
}

.toast-copy {
  display: grid;
  min-width: 0;
  gap: 4px;
  padding-top: 1px;
}

.toast-copy strong {
  overflow: hidden;
  color: var(--color-text-primary);
  font-size: 10px;
  line-height: 1.35;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.toast-copy p {
  display: -webkit-box;
  overflow: hidden;
  margin: 0;
  color: var(--color-text-secondary);
  font-size: 8px;
  line-height: 1.5;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.toast-close,
.toast-action {
  border: 0;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
}

.toast-close {
  display: grid;
  width: 24px;
  height: 24px;
  place-items: center;
  padding: 0;
  font-size: 14px;
}

.toast-close:hover,
.toast-action:hover {
  color: var(--color-text-primary);
}

.toast-action {
  justify-self: start;
  padding: 2px 0 0;
  color: var(--toast-tone);
  font-size: 8px;
}

.toast-stack-enter-active,
.toast-stack-leave-active,
.toast-stack-move {
  transition:
    opacity 190ms ease,
    transform 190ms cubic-bezier(0.2, 0.8, 0.2, 1);
}

.toast-stack-enter-from,
.toast-stack-leave-to {
  opacity: 0;
  transform: translateY(10px);
}

.toast-stack-leave-active {
  position: absolute;
  right: 0;
  bottom: 0;
  width: 100%;
}

@keyframes toast-spin {
  to { transform: rotate(360deg); }
}

@media (prefers-reduced-motion: reduce) {
  .toast-stack-enter-active,
  .toast-stack-leave-active,
  .toast-stack-move {
    transition-duration: 1ms;
  }

  .toast-spinner { animation-duration: 1.4s; }
}

@media (max-width: 680px) {
  .toast-viewport {
    right: 12px;
    bottom: 12px;
    width: calc(100vw - 24px);
  }
}
</style>
