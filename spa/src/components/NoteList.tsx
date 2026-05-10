import { useEffect, useRef, useState } from "preact/hooks";
import { Plus, Trash2, Search, LayoutList, LayoutGrid, X } from "lucide-react";
import { fetchNotes, createNote, deleteNote, connectWs, type Note, type WsEvent } from "../api";
import { notesView } from "../viewMode";
import { selectedNoteId } from "../selectedNote";
import { NoteEditor } from "./NoteEditor";

interface Props { boardId: string }

export function NoteList({ boardId }: Props) {
  const [notes, setNotes] = useState<Note[]>([]);
  const [newText, setNewText] = useState("");
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const stopWs = useRef<(() => void) | null>(null);
  const view = notesView.value;

  useEffect(() => {
    load();
    stopWs.current = connectWs(onWs);
    return () => stopWs.current?.();
  }, [boardId]);

  async function load() {
    try { setNotes(await fetchNotes(boardId)); }
    catch (e) { setError(String(e)); }
  }

  function onWs(e: WsEvent) {
    if (e.event === "note_created" || e.event === "note_deleted") load();
  }

  async function handleAdd(e: Event) {
    e.preventDefault();
    if (!newText.trim()) return;
    setBusy(true);
    try {
      const { id } = await createNote(boardId, newText.trim());
      setNewText("");
      await load();
      selectedNoteId.value = id;
    } catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }

  async function handleDelete(e: Event, id: string) {
    e.stopPropagation();
    if (!confirm("Delete this note?")) return;
    try {
      await deleteNote(id);
      if (selectedNoteId.value === id) selectedNoteId.value = null;
      setNotes((p) => p.filter((n) => n.id !== id));
    } catch (e) { setError(String(e)); }
  }

  const filtered = query.trim()
    ? notes.filter((n) => n.id.includes(query) || n.note_type.includes(query))
    : notes;

  const panelOpen = !!selectedNoteId.value;

  return (
    <div class={`notes-workspace ${panelOpen ? "panel-open" : ""}`}>
      <div class="notes-pane">
        <div class="page-title">
          <h2>Notes</h2>
          <div class="page-title-actions">
            <div class="btn-group">
              <button class={`btn-icon ${view === "list" ? "btn-primary" : ""}`}
                onClick={() => (notesView.value = "list")} title="List view">
                <LayoutList size={15} />
              </button>
              <button class={`btn-icon ${view === "card" ? "btn-primary" : ""}`}
                onClick={() => (notesView.value = "card")} title="Card view">
                <LayoutGrid size={15} />
              </button>
            </div>
          </div>
        </div>

        {error && (
          <div class="error-msg">
            {error}
            <button class="btn-icon" onClick={() => setError(null)}><X size={14} /></button>
          </div>
        )}

        <div class="toolbar">
          <Search size={14} style={{ color: "var(--text-muted)", flexShrink: 0 }} />
          <input type="search" placeholder="Search…" value={query}
            onInput={(e) => setQuery((e.target as HTMLInputElement).value)} />
        </div>

        <form class="toolbar" onSubmit={handleAdd}>
          <input type="text" placeholder="New note…" value={newText}
            onInput={(e) => setNewText((e.target as HTMLInputElement).value)} disabled={busy} />
          <button class="btn-primary" type="submit" disabled={busy || !newText.trim()}>
            <Plus size={14} /> Add
          </button>
        </form>

        {filtered.length === 0 && (
          <p class="empty-msg">{query ? "No matching notes." : "No notes yet."}</p>
        )}

        {view === "list" ? (
          <ul class="item-list">
            {filtered.map((note) => {
              const active = selectedNoteId.value === note.id;
              return (
                <li key={note.id} class={`item-row ${active ? "note-active" : ""}`}
                  onClick={() => (selectedNoteId.value = note.id)} style={{ cursor: "pointer" }}>
                  <div class="item-row-header">
                    <span class="item-name" style={{ fontFamily: "monospace", fontSize: "0.8rem" }}>
                      {note.id.slice(0, 8)}
                    </span>
                    <span style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>{note.note_type}</span>
                    <div class="item-actions" onClick={(e) => e.stopPropagation()}>
                      <button class="btn-icon btn-danger" onClick={(e) => handleDelete(e, note.id)}>
                        <Trash2 size={13} />
                      </button>
                    </div>
                  </div>
                </li>
              );
            })}
          </ul>
        ) : (
          <div class="card-grid">
            {filtered.map((note) => {
              const active = selectedNoteId.value === note.id;
              return (
                <div key={note.id} class={`note-card ${active ? "note-card-active" : ""}`}
                  onClick={() => (selectedNoteId.value = note.id)}>
                  <div class="card-actions" onClick={(e) => e.stopPropagation()}>
                    <button class="btn-icon btn-danger" onClick={(e) => handleDelete(e, note.id)}>
                      <Trash2 size={13} />
                    </button>
                  </div>
                  <span class="note-id">{note.id.slice(0, 8)}…</span>
                  <span style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>{note.note_type}</span>
                </div>
              );
            })}
          </div>
        )}
      </div>

      <NoteEditor onDeleted={(id) => setNotes((p) => p.filter((n) => n.id !== id))} />
    </div>
  );
}
