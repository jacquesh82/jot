import { useEffect, useRef, useState } from "preact/hooks";
import { fetchBoards, createBoard, renameBoard, deleteBoard, type Board } from "../api";

export function BoardList() {
  const [boards, setBoards] = useState<Board[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const renameInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => { load(); }, []);

  useEffect(() => {
    if (renamingId) renameInputRef.current?.focus();
  }, [renamingId]);

  async function load() {
    try {
      setBoards(await fetchBoards());
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleCreate(e: Event) {
    e.preventDefault();
    if (!newName.trim()) return;
    setCreating(true);
    try {
      await createBoard(newName.trim());
      setNewName("");
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  }

  function startRename(b: Board) {
    setRenamingId(b.id);
    setRenameValue(b.name);
  }

  async function commitRename(id: string) {
    if (!renameValue.trim()) { setRenamingId(null); return; }
    try {
      await renameBoard(id, renameValue.trim());
      setRenamingId(null);
      await load();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleDelete(id: string, name: string) {
    if (!confirm(`Delete board "${name}"?`)) return;
    try {
      await deleteBoard(id);
      await load();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div class="page">
      <h2>Boards</h2>
      {error && <div class="error">{error} <button onClick={() => setError(null)}>×</button></div>}

      <form class="inline-form" onSubmit={handleCreate}>
        <input
          type="text"
          placeholder="New board name…"
          value={newName}
          onInput={(e) => setNewName((e.target as HTMLInputElement).value)}
          disabled={creating}
        />
        <button type="submit" disabled={creating || !newName.trim()}>Create</button>
      </form>

      {boards.length === 0 ? (
        <p class="empty">No boards yet.</p>
      ) : (
        <ul class="item-list">
          {boards.map((b) => (
            <li key={b.id} class="item-row">
              {renamingId === b.id ? (
                <input
                  ref={renameInputRef}
                  class="rename-input"
                  value={renameValue}
                  onInput={(e) => setRenameValue((e.target as HTMLInputElement).value)}
                  onBlur={() => commitRename(b.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") commitRename(b.id);
                    if (e.key === "Escape") setRenamingId(null);
                  }}
                />
              ) : (
                <a class="item-name" href={`#/board/${b.id}`}>{b.name}</a>
              )}
              <span class="item-actions">
                <button class="btn-icon" title="Rename" onClick={() => startRename(b)}>✏️</button>
                <button class="btn-icon btn-danger" title="Delete" onClick={() => handleDelete(b.id, b.name)}>🗑</button>
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
