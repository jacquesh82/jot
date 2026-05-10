import { signal } from "@preact/signals";

export const notesView = signal<"list" | "card">(
  (localStorage.getItem("notesView") as "list" | "card") ?? "list"
);
notesView.subscribe((v) => localStorage.setItem("notesView", v));

export const boardsView = signal<"list" | "card">(
  (localStorage.getItem("boardsView") as "list" | "card") ?? "card"
);
boardsView.subscribe((v) => localStorage.setItem("boardsView", v));
