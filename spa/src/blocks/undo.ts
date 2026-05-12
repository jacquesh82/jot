import * as api from "../api";
import type { BlockNode } from "./tree";

export interface UndoEntry {
  /** Short human label, e.g. "delete block", "edit". */
  label: string;
  undo: () => Promise<void>;
  redo: () => Promise<void>;
}

export class UndoStack {
  private undoEntries: UndoEntry[] = [];
  private redoEntries: UndoEntry[] = [];
  private limit: number;
  private busy = false;

  constructor(limit = 100) { this.limit = limit; }

  push(entry: UndoEntry): void {
    if (this.busy) return; // never record actions triggered BY an undo/redo
    this.undoEntries.push(entry);
    if (this.undoEntries.length > this.limit) this.undoEntries.shift();
    // Any new user action invalidates the redo stack.
    this.redoEntries = [];
  }

  canUndo(): boolean { return this.undoEntries.length > 0; }
  canRedo(): boolean { return this.redoEntries.length > 0; }

  async undo(): Promise<void> {
    const e = this.undoEntries.pop();
    if (!e) return;
    this.busy = true;
    try { await e.undo(); this.redoEntries.push(e); }
    finally { this.busy = false; }
  }

  async redo(): Promise<void> {
    const e = this.redoEntries.pop();
    if (!e) return;
    this.busy = true;
    try { await e.redo(); this.undoEntries.push(e); }
    finally { this.busy = false; }
  }

  clear(): void { this.undoEntries = []; this.redoEntries = []; }
}

/** Snapshot of a block subtree as raw (already-encrypted) ciphertext.
 *  Reusing content_b64 verbatim avoids any decrypt/re-encrypt round-trip. */
export interface BlockSnapshot {
  block_type: string;
  content_b64: string;
  metadata_b64: string | null;
  collapsed: boolean;
  children: BlockSnapshot[];
}

export function snapshotSubtree(n: BlockNode): BlockSnapshot {
  return {
    block_type: n.block_type,
    content_b64: n.content,
    metadata_b64: n.metadata ?? null,
    collapsed: n.collapsed,
    children: n.children.map(snapshotSubtree),
  };
}

/** Recreate a block subtree under `parent_id` at `position`.
 *  Returns the new id of the root recreated block. Children get fresh UUIDs. */
export async function recreateSubtree(
  noteId: string,
  snap: BlockSnapshot,
  parent_id: string | null,
  position: number | undefined,
): Promise<string> {
  const created = await api.createBlock(noteId, {
    parent_id,
    position,
    block_type: snap.block_type,
    content_b64: snap.content_b64,
    metadata_b64: snap.metadata_b64,
  });
  for (let i = 0; i < snap.children.length; i++) {
    await recreateSubtree(noteId, snap.children[i], created.id, i);
  }
  if (snap.collapsed) {
    try { await api.patchBlock(created.id, { collapsed: true }); } catch {}
  }
  return created.id;
}
