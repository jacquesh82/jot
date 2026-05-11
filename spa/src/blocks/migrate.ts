import { splitMarkdown } from "./markdown";
import { encryptBlock } from "./crypto";
import * as api from "../api";

/** Convert a legacy markdown note into a tree of blocks and mark schema_version=1.
 *  Idempotent: only call when the note currently has schema_version=0. */
export async function migrateNoteToBlocks(boardId: string, noteId: string, markdown: string): Promise<void> {
  const parts = splitMarkdown(markdown);
  const indentStack: { indent: number; id: string }[] = [];
  for (let i = 0; i < parts.length; i++) {
    const p = parts[i];
    while (indentStack.length && indentStack[indentStack.length - 1].indent >= p.indent) indentStack.pop();
    const parent_id = indentStack.length ? indentStack[indentStack.length - 1].id : null;
    const ct = await encryptBlock(boardId, noteId, p.content);
    const created = await api.createBlock(noteId, {
      parent_id, position: i, block_type: p.block_type, content_b64: ct,
    });
    indentStack.push({ indent: p.indent, id: created.id });
  }
  await api.setNoteSchemaVersion(noteId, 1);
}
