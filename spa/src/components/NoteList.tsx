import { useEffect, useRef, useState } from "preact/hooks";
import {
  fetchNotes, fetchNoteContent, createNote, updateNoteContent,
  deleteNote, connectWs, type Note, type WsEvent,
} from "../api";

interface NoteItem extends Note {
  content?: string;
  contentLoaded?: boolean;
}

interface Props {
  boardId: string;
}

export function NoteList({ boardId }: Props) {
  const [notes, setNotes] = useState<NoteItem[]>([]);
  const [newText, setNewText] = useState("");
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const stopWs = useRef<(() => void) | null>(null);
  const editRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    load();
    stopWs.current = connectWs(handleWsEvent);
    return () => stopWs.current?.();
  }, [boardId]);

  useEffect(() => {
    if (editingId) editRef.current?.focus();
  }, [editingId]);

  async function load() {
    try {
      const fetched = await fetchNotes(boardId);
      setNotes(fetched.map((n) => ({ ...n })));
    } catch (e) {
      setError(String(e));
    }
  }

  function handleWsEvent(e: WsEvent) {
    if (e.event === "note_created" || e.event === "note_deleted") load();
  }

  async function expandNote(note: NoteItem) {
    if (expandedId === note.id) { setExpandedId(null); return; }
    setExpandedId(note.id);
    if (!note.contentLoaded) {
      const content = await fetchNoteContent(note.id);
      setNotes((prev) =>
        prev.map((n) => n.id === note.id ? { ...n, content, contentLoaded: true } : n)
      );
    }
  }

  function startEdit(note: NoteItem) {
    setEditingId(note.id);
    setEditValue(note.content ?? "");
  }

  async function saveEdit(id: string) {
    try {
      await updateNoteContent(id, editValue);
      setNotes((prev) =>
        prev.map((n) => n.id === id ? { ...n, content: editValue, contentLoaded: true } : n)
      );
      setEditingId(null);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleAdd(e: Event) {
    e.preventDefault();
    if (!newText.trim()) return;
    setBusy(true);
    try {
      await createNote(boardId, newText.trim());
      setNewText("");
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete(id: string) {
    if (!confirm("Delete this note?")) return;
    try {
      await deleteNote(id);
      setNotes((prev) => prev.filter((n) => n.id !== id));
    } catch (e) {
      setError(String(e));
    }
  }

  // Search: filter by content if loaded, always show unloaded notes (expand to verify)
  const filtered = query.trim()
    ? notes.filter((n) =>
        n.contentLoaded
          ? n.content?.toLowerCase().includes(query.toLowerCase())
          : true
      )
    : notes;

  return (
    <div class="page">
      <div class="page-header">
        <a class="back-link" href="#/">← Boards</a>
        <h2>Notes</h2>
      </div>

      {error && <div class="error">{error} <button onClick={() => setError(null)}>×</button></div>}

      <input
        class="search-input"
        type="search"
        placeholder="Search notes…"
        value={query}
        onInput={(e) => setQuery((e.target as HTMLInputElement).value)}
      />

      <form class="inline-form" onSubmit={handleAdd}>
        <input
          type="text"
          placeholder="New note…"
          value={newText}
          onInput={(e) => setNewText((e.target as HTMLInputElement).value)}
          disabled={busy}
        />
        <button type="submit" disabled={busy || !newText.trim()}>Add</button>
      </form>

      {filtered.length === 0 ? (
        <p class="empty">{query ? "No matching notes." : "No notes in this board."}</p>
      ) : (
        <ul class="item-list">
          {filtered.map((note) => (
            <li key={note.id} class={`item-row note-row ${expandedId === note.id ? "expanded" : ""}`}>
              <div class="note-header" onClick={() => expandNote(note)}>
                <span class="note-preview">
                  {note.contentLoaded && note.content
                    ? note.content.slice(0, 60) + (note.content.length > 60 ? "…" : "")
                    : <span class="note-id">{note.id.slice(0, 8)}</span>}
                </span>
                <span class="item-actions" onClick={(e) => e.stopPropagation()}>
                  <button class="btn-icon" title="Edit" onClick={() => { setExpandedId(note.id); startEdit(note); }}>✏️</button>
                  <button class="btn-icon btn-danger" title="Delete" onClick={() => handleDelete(note.id)}>🗑</button>
                </span>
              </div>

              {expandedId === note.id && (
                <div class="note-body">
                  {editingId === note.id ? (
                    <>
                      <textarea
                        ref={editRef}
                        class="note-editor"
                        value={editValue}
                        onInput={(e) => setEditValue((e.target as HTMLTextAreaElement).value)}
                        rows={6}
                      />
                      <div class="note-edit-actions">
                        <button class="btn-primary" onClick={() => saveEdit(note.id)}>Save</button>
                        <button onClick={() => setEditingId(null)}>Cancel</button>
                      </div>
                    </>
                  ) : (
                    <pre class="note-content" onDblClick={() => startEdit(note)}>
                      {note.content ?? "Loading…"}
                    </pre>
                  )}
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
