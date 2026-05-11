import { useEffect, useState } from "preact/hooks";
import type { JSX } from "preact";
import * as api from "../api";
import { buildTree, flatten, type BlockNode } from "../blocks/tree";
import { decryptBlock } from "../blocks/crypto";
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
    <div class={`block-row ${active === n.id ? "active" : ""}`} style={{ paddingLeft: `${depth * 24}px` }} key={n.id}>
      <span class="block-bullet" data-id={n.id}>•</span>
      <div
        class="block-content"
        contentEditable
        onFocus={() => setActive(n.id)}
        onBlur={(ev) => onBlur(n, (ev.target as HTMLElement).innerText)}
        onKeyDown={onKeyDown as any}
        dangerouslySetInnerHTML={{ __html: escapeHtml(n.plaintext) }}
      />
      {!n.collapsed && n.children.map(c => renderNode(c, depth + 1))}
    </div>
  );

  return (
    <div class="block-editor">
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
