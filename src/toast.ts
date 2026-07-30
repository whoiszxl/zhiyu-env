import { ref } from "vue";

export type ToastIntent = "success" | "info" | "warning" | "error" | "progress";

export interface ToastInput {
  intent: ToastIntent;
  title: string;
  message?: string;
  key?: string;
  duration?: number;
  actionLabel?: string;
  onAction?: () => void;
}

export interface ToastItem extends ToastInput {
  id: number;
  remainingMs: number;
}

export const toasts = ref<ToastItem[]>([]);

let nextToastId = 1;
const timers = new Map<number, ReturnType<typeof setTimeout>>();
const deadlines = new Map<number, number>();

function defaultDuration(intent: ToastIntent): number {
  if (intent === "success") return 3_200;
  if (intent === "info") return 4_500;
  if (intent === "warning") return 6_500;
  return 0;
}

function clearTimer(id: number) {
  const timer = timers.get(id);
  if (timer) clearTimeout(timer);
  timers.delete(id);
  deadlines.delete(id);
}

function schedule(toast: ToastItem, duration: number) {
  clearTimer(toast.id);
  toast.remainingMs = duration;
  if (duration <= 0) return;
  deadlines.set(toast.id, Date.now() + duration);
  timers.set(
    toast.id,
    setTimeout(() => dismissToast(toast.id), duration),
  );
}

export function dismissToast(id: number) {
  clearTimer(id);
  const index = toasts.value.findIndex((toast) => toast.id === id);
  if (index >= 0) toasts.value.splice(index, 1);
}

export function dismissToastByKey(key: string) {
  const toast = toasts.value.find((item) => item.key === key);
  if (toast) dismissToast(toast.id);
}

export function pauseToast(id: number) {
  const toast = toasts.value.find((item) => item.id === id);
  const deadline = deadlines.get(id);
  if (!toast || !deadline) return;
  toast.remainingMs = Math.max(250, deadline - Date.now());
  clearTimer(id);
}

export function resumeToast(id: number) {
  const toast = toasts.value.find((item) => item.id === id);
  if (!toast || toast.remainingMs <= 0 || timers.has(id)) return;
  schedule(toast, toast.remainingMs);
}

export function showToast(input: ToastInput): number {
  const existing = input.key
    ? toasts.value.find((toast) => toast.key === input.key)
    : undefined;
  const duration = input.duration ?? defaultDuration(input.intent);

  if (existing) {
    Object.assign(existing, {
      message: undefined,
      actionLabel: undefined,
      onAction: undefined,
      ...input,
    });
    schedule(existing, duration);
    return existing.id;
  }

  const toast: ToastItem = {
    ...input,
    id: nextToastId++,
    remainingMs: duration,
  };
  toasts.value.push(toast);

  while (toasts.value.length > 3) {
    dismissToast(toasts.value[0].id);
  }

  schedule(toast, duration);
  return toast.id;
}

export const toast = {
  success: (title: string, options: Omit<ToastInput, "intent" | "title"> = {}) =>
    showToast({ ...options, intent: "success", title }),
  info: (title: string, options: Omit<ToastInput, "intent" | "title"> = {}) =>
    showToast({ ...options, intent: "info", title }),
  warning: (title: string, options: Omit<ToastInput, "intent" | "title"> = {}) =>
    showToast({ ...options, intent: "warning", title }),
  error: (title: string, options: Omit<ToastInput, "intent" | "title"> = {}) =>
    showToast({ ...options, intent: "error", title }),
  progress: (title: string, options: Omit<ToastInput, "intent" | "title"> = {}) =>
    showToast({ ...options, intent: "progress", title, duration: 0 }),
};
