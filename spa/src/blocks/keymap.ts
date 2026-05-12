import * as api from "../api";
import { encryptBlock } from "./crypto";
import type { BlockNode } from "./tree";

export interface KeymapCtx {
  noteId: string;
  boardId: string;
  blocks: BlockNode[];        // flat list, depth-first
  activeIdx: number;
  refresh: () => Promise<void>;
  setActive: (id: string) => void;
}

export async function newBlockBelow(ctx: KeymapCtx) {
  const cur = ctx.blocks[ctx.activeIdx];
  const ct = await encryptBlock(ctx.boardId, ctx.noteId, "");
  const created = await api.createBlock(ctx.noteId, {
    parent_id: cur?.parent_block_id ?? null,
    position: cur ? cur.position + 0.5 : undefined,
    block_type: "text",
    content_b64: ct,
  });
  await ctx.refresh();
  ctx.setActive(created.id);
}

function precedingSibling(blocks: BlockNode[], cur: BlockNode): BlockNode | null {
  const siblings = blocks.filter(b => b.parent_block_id === cur.parent_block_id);
  const idx = siblings.findIndex(b => b.id === cur.id);
  return idx > 0 ? siblings[idx - 1] : null;
}

export async function indent(ctx: KeymapCtx) {
  const cur = ctx.blocks[ctx.activeIdx];
  if (!cur) return;
  if (!precedingSibling(ctx.blocks, cur)) return; // silent no-op: first child at this level
  try { await api.indentBlock(cur.id); } catch (e) { console.warn("indent failed", e); return; }
  await ctx.refresh();
}

export async function outdent(ctx: KeymapCtx) {
  const cur = ctx.blocks[ctx.activeIdx];
  if (!cur) return;
  if (!cur.parent_block_id) return; // silent no-op: already at root
  try { await api.outdentBlock(cur.id); } catch (e) { console.warn("outdent failed", e); return; }
  await ctx.refresh();
}

export async function deleteActive(ctx: KeymapCtx) {
  const cur = ctx.blocks[ctx.activeIdx];
  if (!cur) return;
  await api.deleteBlock(cur.id);
  await ctx.refresh();
}

export async function persistEdit(
  boardId: string,
  noteId: string,
  blockId: string,
  plaintext: string,
  block_type?: string,
) {
  const ct = await encryptBlock(boardId, noteId, plaintext);
  await api.patchBlock(blockId, { content_b64: ct, block_type });
}
