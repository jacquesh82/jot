import { signal } from "@preact/signals";

const stored = (localStorage.getItem("theme") ?? "light") as "light" | "dark";
export const theme = signal<"light" | "dark">(stored);

document.documentElement.setAttribute("data-theme", theme.value);
theme.subscribe((val) => {
  document.documentElement.setAttribute("data-theme", val);
  localStorage.setItem("theme", val);
});

export function toggleTheme() {
  theme.value = theme.value === "light" ? "dark" : "light";
}
