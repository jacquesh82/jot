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
  ctx.setActive(cur.id);
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
  ctx.setActive(cur.id);
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
  ctx.setActive(blockId);
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
  ctx.setActive(blockId);
}

// kept for type completeness
export { siblingPosition };

// ────────────────────────────────────────────────────────────────────────────
// Multi-selection batch operations
// ────────────────────────────────────────────────────────────────────────────

/** Remove from the id list any block whose ancestor is also present in the list. */
function filterOutDescendants(blocks: BlockNode[], ids: string[]): string[] {
  const set = new Set(ids);
  const byId = new Map(blocks.map(b => [b.id, b]));
  return ids.filter(id => {
    let cur = byId.get(id);
    while (cur?.parent_block_id) {
      if (set.has(cur.parent_block_id)) return false;
      cur = byId.get(cur.parent_block_id);
    }
    return true;
  });
}

export async function deleteMany(ctx: KeymapCtx, ids: string[]) {
  const dedup = filterOutDescendants(ctx.blocks, ids);
  if (dedup.length === 0) return;
  const byId = new Map(ctx.blocks.map(b => [b.id, b]));

  // Snapshot pre-deletion state so undo can recreate the subtrees.
  const snaps: { snap: ReturnType<typeof snapshotSubtree>; parent_id: string | null; position: number }[] = [];
  for (const id of dedup) {
    const node = byId.get(id);
    if (!node) continue;
    snaps.push({ snap: snapshotSubtree(node), parent_id: node.parent_block_id, position: node.position });
  }

  for (const id of dedup) {
    try { await api.deleteBlock(id); } catch {}
  }

  const ref = { ids: [] as string[] };
  ctx.undoStack.push({
    label: `delete ${snaps.length} blocks`,
    undo: async () => {
      const newIds: string[] = [];
      for (const s of snaps) {
        const nid = await recreateSubtree(ctx.noteId, s.snap, s.parent_id, s.position);
        newIds.push(nid);
      }
      ref.ids = newIds;
      await ctx.refresh();
    },
    redo: async () => {
      for (const id of ref.ids) {
        try { await api.deleteBlock(id); } catch {}
      }
      await ctx.refresh();
    },
  });
  await ctx.refresh();
}

export async function indentMany(ctx: KeymapCtx, ids: string[]) {
  const byId = new Map(ctx.blocks.map(b => [b.id, b]));
  const ops: { id: string; prevParent: string | null; prevPosition: number }[] = [];
  for (const id of ids) {
    const b = byId.get(id);
    if (!b) continue;
    if (!precedingSibling(ctx.blocks, b)) continue; // can't indent first child
    ops.push({ id, prevParent: b.parent_block_id, prevPosition: b.position });
    try { await api.indentBlock(id); } catch {}
  }
  if (ops.length === 0) return;
  ctx.undoStack.push({
    label: `indent ${ops.length} blocks`,
    undo: async () => {
      for (let i = ops.length - 1; i >= 0; i--) {
        const o = ops[i];
        try { await api.moveBlock(o.id, o.prevParent, o.prevPosition); } catch {}
      }
      await ctx.refresh();
    },
    redo: async () => {
      for (const o of ops) {
        try { await api.indentBlock(o.id); } catch {}
      }
      await ctx.refresh();
    },
  });
  await ctx.refresh();
}

export async function outdentMany(ctx: KeymapCtx, ids: string[]) {
  const byId = new Map(ctx.blocks.map(b => [b.id, b]));
  const ops: { id: string; prevParent: string | null; prevPosition: number }[] = [];
  for (const id of ids) {
    const b = byId.get(id);
    if (!b) continue;
    if (!b.parent_block_id) continue; // already root
    ops.push({ id, prevParent: b.parent_block_id, prevPosition: b.position });
    try { await api.outdentBlock(id); } catch {}
  }
  if (ops.length === 0) return;
  ctx.undoStack.push({
    label: `outdent ${ops.length} blocks`,
    undo: async () => {
      for (let i = ops.length - 1; i >= 0; i--) {
        const o = ops[i];
        try { await api.moveBlock(o.id, o.prevParent, o.prevPosition); } catch {}
      }
      await ctx.refresh();
    },
    redo: async () => {
      for (const o of ops) {
        try { await api.outdentBlock(o.id); } catch {}
      }
      await ctx.refresh();
    },
  });
  await ctx.refresh();
}
