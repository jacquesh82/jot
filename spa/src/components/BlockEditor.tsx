import { useEffect, useState } from "preact/hooks";
import type { JSX } from "preact";
import * as api from "../api";
import { buildTree, flatten, type BlockNode } from "../blocks/tree";
import { decryptBlock, encryptBlock } from "../blocks/crypto";
import * as keymap from "../blocks/keymap";
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

  const ctx = (): keymap.KeymapCtx => ({
    noteId, boardId,
    blocks: flat,
    activeIdx: Math.max(0, flat.findIndex(b => b.id === active)),
    refresh, setActive,
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
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); await keymap.newBlockBelow(ctx()); }
    else if (e.key === "Tab" && !e.shiftKey) { e.preventDefault(); await keymap.indent(ctx()); }
    else if (e.key === "Tab" &&  e.shiftKey) { e.preventDefault(); await keymap.outdent(ctx()); }
    else if (e.key === "Backspace") {
      const cur = ctx().blocks[ctx().activeIdx];
      if (cur && (e.target as HTMLElement).innerText.trim() === "") {
        e.preventDefault(); await keymap.deleteActive(ctx());
      }
    }
  };

  const onBlur = async (b: BlockNode, text: string) => {
    if (text !== b.plaintext) {
      await keymap.persistEdit(boardId, noteId, b.id, text);
      await refresh();
    }
  };

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
              await api.moveBlock(draggedId, n.parent_block_id, n.position + 0.5);
              await refresh();
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
      <input
        class="block-title"
        type="text"
        value={title}
        onInput={(e) => setTitle((e.target as HTMLInputElement).value)}
        onBlur={() => saveTitle()}
        placeholder={t("block.title_placeholder")}
      />
      {roots.length === 0 ? <p class="block-empty">{t("block.empty")}</p> : roots.map(r => renderNode(r))}
      {slashFor && (
        <SlashMenu
          onClose={() => setSlashFor(null)}
          onPick={async (id) => {
            try { await api.patchBlock(slashFor!, { block_type: id }); }
            catch (e) { console.warn("patch type failed", e); }
            setSlashFor(null);
            await refresh();
          }}
        />
      )}
    </div>
  );
}
