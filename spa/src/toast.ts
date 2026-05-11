import { signal } from "@preact/signals";

export type ToastType = "success" | "error" | "warn" | "info";

export interface Toast {
  id: number;
  message: string;
  type: ToastType;
}

export const toasts = signal<Toast[]>([]);

let _id = 0;

export function toast(message: string, type: ToastType = "info", duration = 3500) {
  const id = ++_id;
  toasts.value = [...toasts.value, { id, message, type }];
  setTimeout(() => dismiss(id), duration);
}

export function dismiss(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id);
}
