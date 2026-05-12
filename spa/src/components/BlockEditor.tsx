import { useEffect, useMemo, useState } from "preact/hooks";
import type { JSX } from "preact";
import * as api from "../api";
import { buildTree, flatten, type BlockNode } from "../blocks/tree";
import { decryptBlock, encryptBlock } from "../blocks/crypto";
import * as keymap from "../blocks/keymap";
import { UndoStack } from "../blocks/undo";
import { t } from "../i18n";
import { SlashMenu } from "./SlashMenu";
import "./BlockEditor.css";

interface Props { noteId: string; boardId: string; }

export function BlockEditor({ noteId, boardId }: Props) {
  const [roots, setRoots] = useState<BlockNode[]>([]);
  const [flat, setFlat] = useState<BlockNode[]>([]);
  const [active, setActive] = useState<string | null>(null);
  const [slashFor, setSlashFor] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved">("idle");
  // One stack per note open: switching notes resets undo history.
  const undoStack = useMemo(() => new UndoStack(), [noteId]);

  // Wrap any async mutation so the badge reflects "saving…" → "saved".
  const withSaving = async <T,>(fn: () => Promise<T>): Promise<T> => {
    setSaveState("saving");
    try {
      const r = await fn();
      setSaveState("saved");
      window.setTimeout(() => setSaveState(s => (s === "saved" ? "idle" : s)), 1200);
      return r;
    } catch (err) {
      setSaveState("idle");
      throw err;
    }
  };

  // Focus a block's contentEditable and place the caret at the end.
  // Scheduled on rAF so it runs AFTER the next render (refresh() triggers
  // a re-render that destroys the previously-focused node).
  const focusBlock = (id: string, attempts = 3) => {
    requestAnimationFrame(() => {
      const bullet = document.querySelector(`.block-bullet[data-id="${CSS.escape(id)}"]`);
      const content = bullet?.parentElement?.querySelector(".block-content") as HTMLElement | null;
      if (!content) {
        if (attempts > 0) focusBlock(id, attempts - 1);
        return;
      }
      content.focus();
      const range = document.createRange();
      range.selectNodeContents(content);
      range.collapse(false);
      const sel = window.getSelection();
      sel?.removeAllRanges();
      sel?.addRange(range);
    });
  };

  useEffect(() => {
    (async () => {
      try {
        const meta = await api.fetchNoteMeta(noteId);
        if (meta.title_b64) {
          try { setTitle(await decryptBlock(boardId, noteId, meta.title_b64)); }
          catch { setTitle(""); }
        } else {
          setTitle("");
        }
      } catch { setTitle(""); }
    })();
  }, [noteId, boardId]);

  const saveTitle = async () => {
    try {
      if (title.trim() === "") {
        await api.patchNoteTitle(noteId, null);
        return;
      }
      const ct = await encryptBlock(boardId, noteId, title);
      await api.patchNoteTitle(noteId, ct);
    } catch (e) { console.warn("save title failed", e); }
  };

  const refresh = async () => {
    const dtos = await api.listBlocks(noteId);
    const tree = buildTree(dtos);
    const all = flatten(tree);
    await Promise.all(all.map(async n => {
      try { n.plaintext = await decryptBlock(boardId, noteId, n.content); }
      catch { n.plaintext = ""; }
    }));
    setRoots(tree);
    setFlat(all);
  };

  useEffect(() => { refresh(); }, [noteId]);

  useEffect(() => {
    const off = api.connectWs((evt) => {
      const e = evt?.event;
      if (typeof e !== "string" || !e.startsWith("block_")) return;
      if ((evt as any).note_id !== noteId) return;
      refresh();
    });
    return () => off();
  }, [noteId]);

  // Global Ctrl/Cmd+Z and Ctrl/Cmd+Shift+Z handler at the editor level.
  // Skips when a contentEditable block is focused so the browser's native
  // intra-block text undo still works.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const isMod = e.ctrlKey || e.metaKey;
      if (!isMod) return;
      const key = e.key.toLowerCase();
      if (key !== "z" && key !== "y") return;
      const target = e.target as HTMLElement | null;
      const inEditable = target?.closest(".block-content[contenteditable=\"true\"]") != null;
      if (inEditable) return; // let the browser handle inline text undo
      e.preventDefault();
      const isRedo = (key === "z" && e.shiftKey) || key === "y";
      (isRedo ? undoStack.redo() : undoStack.undo()).catch(err => console.warn("undo/redo failed", err));
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [undoStack]);

  const ctx = (): keymap.KeymapCtx => ({
    noteId, boardId,
    blocks: flat,
    activeIdx: Math.max(0, flat.findIndex(b => b.id === active)),
    refresh,
    setActive: (id: string) => { setActive(id); focusBlock(id); },
    undoStack,
  });

  const onKeyDown = async (e: KeyboardEvent) => {
    if (e.key === "/" && (e.target as HTMLElement).innerText.trim() === "") {
      e.preventDefault();
      const row = (e.target as HTMLElement).closest(".block-row");
      const bullet = row?.querySelector(".block-bullet") as HTMLElement | null;
      const id = bullet?.getAttribute("data-id");
      if (id) setSlashFor(id);
      return;
    }
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); await withSaving(() => keymap.newBlockBelow(ctx())); }
    else if (e.key === "Tab" && !e.shiftKey) { e.preventDefault(); await withSaving(() => keymap.indent(ctx())); }
    else if (e.key === "Tab" &&  e.shiftKey) { e.preventDefault(); await withSaving(() => keymap.outdent(ctx())); }
    else if (e.key === "Backspace") {
      const cur = ctx().blocks[ctx().activeIdx];
      if (cur && (e.target as HTMLElement).innerText.trim() === "") {
        e.preventDefault(); await withSaving(() => keymap.deleteActive(ctx()));
      }
    }
  };

  const onBlur = async (b: BlockNode, text: string) => {
    if (text !== b.plaintext) {
      await withSaving(async () => {
        await keymap.persistEdit(ctx(), b, text);
        await refresh();
      });
    }
  };

  const saveTitleTracked = async () => withSaving(saveTitle);

  const escapeHtml = (s: string): string =>
    s.replace(/[&<>]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" }[c]!));

  const renderNode = (n: BlockNode, depth = 0): JSX.Element => (
    <div class={`block-row ${active === n.id ? "active" : ""}`} key={n.id}>
      <div class="block-line" style={{ paddingLeft: `${depth * 24}px` }}>
        <span
          class="block-bullet"
          data-id={n.id}
          draggable
          onDragStart={(e) => {
            (e as DragEvent).dataTransfer!.setData("text/block-id", n.id);
          }}
          onDragOver={(e) => {
            e.preventDefault();
            (e.currentTarget as HTMLElement).classList.add("drop-target");
          }}
          onDragLeave={(e) => (e.currentTarget as HTMLElement).classList.remove("drop-target")}
          onDrop={async (e) => {
            e.preventDefault();
            (e.currentTarget as HTMLElement).classList.remove("drop-target");
            const draggedId = (e as DragEvent).dataTransfer!.getData("text/block-id");
            if (!draggedId || draggedId === n.id) return;
            try {
              await withSaving(() => keymap.moveBlockTo(ctx(), draggedId, n.parent_block_id, n.position + 0.5));
            } catch (err) { console.warn("move failed", err); }
          }}
        >•</span>
        <div
          class="block-content"
          contentEditable
          onFocus={() => setActive(n.id)}
          onBlur={(ev) => onBlur(n, (ev.target as HTMLElement).innerText)}
          onKeyDown={onKeyDown as any}
          dangerouslySetInnerHTML={{ __html: escapeHtml(n.plaintext) }}
        />
      </div>
      {!n.collapsed && n.children.length > 0 && (
        <div class="block-children">
          {n.children.map(c => renderNode(c, depth + 1))}
        </div>
      )}
    </div>
  );

  return (
    <div class="block-editor">
      <div class="block-header">
        <input
          class="block-title"
          type="text"
          value={title}
          onInput={(e) => setTitle((e.target as HTMLInputElement).value)}
          onBlur={() => saveTitleTracked()}
          placeholder={t("block.title_placeholder")}
        />
        <span class={`block-save-state save-${saveState}`} aria-live="polite">
          {saveState === "saving" ? t("block.saving") : saveState === "saved" ? t("block.saved") : t("block.autosave")}
        </span>
      </div>
      {roots.length === 0 ? <p class="block-empty">{t("block.empty")}</p> : roots.map(r => renderNode(r))}
      {slashFor && (
        <SlashMenu
          onClose={() => setSlashFor(null)}
          onPick={async (id) => {
            try { await withSaving(() => keymap.changeType(ctx(), slashFor!, id)); }
            catch (e) { console.warn("patch type failed", e); }
            setSlashFor(null);
          }}
        />
      )}
    </div>
  );
}
