import { useEffect, useRef, useState } from "preact/hooks";
import { t } from "../i18n";

export type SlashId = "text" | "heading" | "todo" | "quote" | "code" | "divider";
export interface SlashItem { id: SlashId; }

interface Props {
  onPick: (id: SlashId) => void;
  onClose: () => void;
}

const ITEMS: SlashItem[] = [
  { id: "heading" },
  { id: "todo" },
  { id: "quote" },
  { id: "code" },
  { id: "divider" },
  { id: "text" },
];

export function SlashMenu({ onPick, onClose }: Props) {
  const [active, setActive] = useState(0);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowDown") { e.preventDefault(); setActive(a => (a + 1) % ITEMS.length); }
      else if (e.key === "ArrowUp") { e.preventDefault(); setActive(a => (a - 1 + ITEMS.length) % ITEMS.length); }
      else if (e.key === "Enter") { e.preventDefault(); onPick(ITEMS[active].id); }
      else if (e.key === "Escape") { e.preventDefault(); onClose(); }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [active]);

  return (
    <div class="slash-menu" ref={ref}>
      {ITEMS.map((it, i) => (
        <button
          class={i === active ? "active" : ""}
          onMouseEnter={() => setActive(i)}
          onClick={() => onPick(it.id)}
          key={it.id}
        >
          {t(`block.type.${it.id}`)}
        </button>
      ))}
    </div>
  );
}
