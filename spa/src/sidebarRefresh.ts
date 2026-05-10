import { signal } from "@preact/signals";

export const sidebarVersion = signal(0);

export function refreshSidebar() {
  sidebarVersion.value += 1;
}
