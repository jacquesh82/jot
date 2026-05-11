import { CheckCircle, AlertCircle, AlertTriangle, Info, X } from "lucide-react";
import { toasts, dismiss, type ToastType } from "../toast";

const ICONS: Record<ToastType, typeof CheckCircle> = {
  success: CheckCircle,
  error:   AlertCircle,
  warn:    AlertTriangle,
  info:    Info,
};

export function ToastContainer() {
  const list = toasts.value;
  if (list.length === 0) return null;

  return (
    <div class="toast-container">
      {list.map((t) => {
        const Icon = ICONS[t.type];
        return (
          <div key={t.id} class={`toast toast-${t.type}`}>
            <Icon size={15} class="toast-icon" />
            <span class="toast-msg">{t.message}</span>
            <button class="toast-close" onClick={() => dismiss(t.id)} aria-label="Dismiss">
              <X size={13} />
            </button>
          </div>
        );
      })}
    </div>
  );
}
