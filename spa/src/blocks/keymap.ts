import * as api from "../api";
import { encryptBlock } from "./crypto";
import type { BlockNode } from "./tree";
import { snapshotSubtree, recreateSubtree, type UndoStack } from "./undo";

export interface KeymapCtx {
  noteId: string;
  boardId: string;
  blocks: BlockNode[];        // flat list, depth-first
  activeIdx: number;
  refresh: () => Promise<void>;
  setActive: (id: string) => void;
  undoStack: UndoStack;
}

function siblingPosition(blocks: BlockNode[], parent_id: string | null, after: BlockNode | null): number | undefined {
  if (!after) return undefined;
  return after.position + 0.5;
  void parent_id; void blocks;
}

export async function newBlockBelow(ctx: KeymapCtx) {
  const cur = ctx.blocks[ctx.activeIdx];
  const parent_id = cur?.parent_block_id ?? null;
  const position = cur ? cur.position + 0.5 : undefined;
  const ct = await encryptBlock(ctx.boardId, ctx.noteId, "");
  const created = await api.createBlock(ctx.noteId, {
    parent_id, position, block_type: "text", content_b64: ct,
  });
  ctx.undoStack.push({
    label: "new block",
    undo: async () => { try { await api.deleteBlock(created.id); } catch {} await ctx.refresh(); },
    redo: async () => {
      const again = await api.createBlock(ctx.noteId, {
        parent_id, position, block_type: "text", content_b64: ct,
      });
      // The recreated block has a NEW id — patch the entry so a second undo still works.
      created.id = again.id;
      await ctx.refresh();
    },
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
  if (!precedingSibling(ctx.blocks, cur)) return;
  const prevParent = cur.parent_block_id;
  const prevPosition = cur.position;
  try { await api.indentBlock(cur.id); }
  catch (e) { console.warn("indent failed", e); return; }
  ctx.undoStack.push({
    label: "indent",
    undo: async () => { await api.moveBlock(cur.id, prevParent, prevPosition); await ctx.refresh(); },
    redo: async () => { try { await api.indentBlock(cur.id); } catch {} await ctx.refresh(); },
  });
  await ctx.refresh();
}

export async function outdent(ctx: KeymapCtx) {
  const cur = ctx.blocks[ctx.activeIdx];
  if (!cur) return;
  if (!cur.parent_block_id) return;
  const prevParent = cur.parent_block_id;
  const prevPosition = cur.position;
  try { await api.outdentBlock(cur.id); }
  catch (e) { console.warn("outdent failed", e); return; }
  ctx.undoStack.push({
    label: "outdent",
    undo: async () => { await api.moveBlock(cur.id, prevParent, prevPosition); await ctx.refresh(); },
    redo: async () => { try { await api.outdentBlock(cur.id); } catch {} await ctx.refresh(); },
  });
  await ctx.refresh();
}

export async function deleteActive(ctx: KeymapCtx) {
  const cur = ctx.blocks[ctx.activeIdx];
  if (!cur) return;
  // Find the in-tree node (with children) — `flat` may be a flattened reference
  // but BlockNode.children is populated by buildTree, so cur.children works.
  const snap = snapshotSubtree(cur);
  const prevParent = cur.parent_block_id;
  const prevPosition = cur.position;
  const ref = { id: cur.id };

  await api.deleteBlock(cur.id);
  ctx.undoStack.push({
    label: "delete block",
    undo: async () => {
      const newId = await recreateSubtree(ctx.noteId, snap, prevParent, prevPosition);
      ref.id = newId; // so a subsequent redo deletes the freshly recreated block
      await ctx.refresh();
    },
    redo: async () => { try { await api.deleteBlock(ref.id); } catch {} await ctx.refresh(); },
  });
  await ctx.refresh();
}

export async function persistEdit(
  ctx: KeymapCtx,
  block: BlockNode,
  plaintext: string,
  block_type?: string,
) {
  const newCt = await encryptBlock(ctx.boardId, ctx.noteId, plaintext);
  const oldCt = block.content;
  const oldType = block.block_type;
  const newType = block_type ?? oldType;
  await api.patchBlock(block.id, { content_b64: newCt, block_type: newType });
  ctx.undoStack.push({
    label: "edit",
    undo: async () => {
      await api.patchBlock(block.id, { content_b64: oldCt, block_type: oldType });
      await ctx.refresh();
    },
    redo: async () => {
      await api.patchBlock(block.id, { content_b64: newCt, block_type: newType });
      await ctx.refresh();
    },
  });
}

export async function changeType(ctx: KeymapCtx, blockId: string, newType: string) {
  const block = ctx.blocks.find(b => b.id === blockId);
  if (!block) return;
  const oldType = block.block_type;
  if (oldType === newType) return;
  await api.patchBlock(blockId, { block_type: newType });
  ctx.undoStack.push({
    label: "change type",
    undo: async () => { await api.patchBlock(blockId, { block_type: oldType }); await ctx.refresh(); },
    redo: async () => { await api.patchBlock(blockId, { block_type: newType }); await ctx.refresh(); },
  });
  await ctx.refresh();
}

export async function moveBlockTo(
  ctx: KeymapCtx,
  blockId: string,
  newParent: string | null,
  newPosition: number,
) {
  const block = ctx.blocks.find(b => b.id === blockId);
  if (!block) return;
  const prevParent = block.parent_block_id;
  const prevPosition = block.position;
  await api.moveBlock(blockId, newParent, newPosition);
  ctx.undoStack.push({
    label: "move",
    undo: async () => { await api.moveBlock(blockId, prevParent, prevPosition); await ctx.refresh(); },
    redo: async () => { await api.moveBlock(blockId, newParent, newPosition); await ctx.refresh(); },
  });
  await ctx.refresh();
}

// kept for type completeness
export { siblingPosition };
