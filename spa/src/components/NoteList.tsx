import { useEffect, useRef, useState } from "preact/hooks";
import { fetchNotes, createNote, deleteNote, connectWs, type Note, type WsEvent } from "../api";

interface Props {
  boardId: string;
}

export function NoteList({ boardId }: Props) {
  const [notes, setNotes] = useState<Note[]>([]);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const stopWs = useRef<(() => void) | null>(null);

  useEffect(() => {
    load();
    stopWs.current = connectWs(handleWsEvent);
    return () => stopWs.current?.();
  }, [boardId]);

  async function load() {
    try {
      setNotes(await fetchNotes(boardId));
    } catch (e) {
      setError(String(e));
    }
  }

  function handleWsEvent(e: WsEvent) {
    if (e.event === "note_created" || e.event === "note_deleted") {
      load();
    }
  }

  async function handleAdd(e: Event) {
    e.preventDefault();
    if (!text.trim()) return;
    setBusy(true);
    try {
      await createNote(boardId, text.trim());
      setText("");
      await load();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete(id: string) {
    try {
      await deleteNote(id);
      await load();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div class="note-list">
      <div class="note-list__header">
        <a href="#/">← Boards</a>
        <h2>Notes</h2>
      </div>
      {error && <div class="error">{error}</div>}
      <form onSubmit={handleAdd} class="note-list__form">
        <input
          type="text"
          placeholder="New note…"
          value={text}
          onInput={(e) => setText((e.target as HTMLInputElement).value)}
          disabled={busy}
        />
        <button type="submit" disabled={busy || !text.trim()}>Add</button>
      </form>
      {notes.length === 0 ? (
        <p class="empty">No notes in this board.</p>
      ) : (
        <ul>
          {notes.map((n) => (
            <li key={n.id} class="note-item">
              <span class="note-item__id">{n.id.slice(0, 8)}</span>
              <span class="note-item__type">{n.note_type}</span>
              <button
                class="note-item__delete"
                onClick={() => handleDelete(n.id)}
                aria-label="Delete note"
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
