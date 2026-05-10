import { useEffect, useRef, useState } from "preact/hooks";
import { Plus, Trash2, Pencil, Search, LayoutList, LayoutGrid, Check, X, ChevronDown, ChevronRight } from "lucide-react";
import { fetchNotes, fetchNoteContent, createNote, updateNoteContent, deleteNote, connectWs, type Note, type WsEvent } from "../api";
import { notesView } from "../viewMode";

interface NoteItem extends Note { content?: string; loaded?: boolean }

interface Props { boardId: string }

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
  const view = notesView.value;

  useEffect(() => {
    load();
    stopWs.current = connectWs(onWs);
    return () => stopWs.current?.();
  }, [boardId]);

  useEffect(() => { if (editingId) editRef.current?.focus(); }, [editingId]);

  async function load() {
    try { setNotes((await fetchNotes(boardId)).map((n) => ({ ...n }))); }
    catch (e) { setError(String(e)); }
  }

  function onWs(e: WsEvent) {
    if (e.event === "note_created" || e.event === "note_deleted") load();
  }

  async function expand(note: NoteItem) {
    if (expandedId === note.id) { setExpandedId(null); return; }
    setExpandedId(note.id);
    if (!note.loaded) {
      const content = await fetchNoteContent(note.id);
      setNotes((p) => p.map((n) => n.id === note.id ? { ...n, content, loaded: true } : n));
    }
  }

  function startEdit(note: NoteItem) {
    setExpandedId(note.id);
    setEditingId(note.id);
    setEditValue(note.content ?? "");
  }

  async function saveEdit(id: string) {
    try {
      await updateNoteContent(id, editValue);
      setNotes((p) => p.map((n) => n.id === id ? { ...n, content: editValue, loaded: true } : n));
      setEditingId(null);
    } catch (e) { setError(String(e)); }
  }

  async function handleAdd(e: Event) {
    e.preventDefault();
    if (!newText.trim()) return;
    setBusy(true);
    try { await createNote(boardId, newText.trim()); setNewText(""); await load(); }
    catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }

  async function handleDelete(id: string) {
    if (!confirm("Delete this note?")) return;
    try { await deleteNote(id); setNotes((p) => p.filter((n) => n.id !== id)); }
    catch (e) { setError(String(e)); }
  }

  const filtered = query.trim()
    ? notes.filter((n) => n.loaded ? n.content?.toLowerCase().includes(query.toLowerCase()) : true)
    : notes;

  return (
    <div>
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
        <input type="search" placeholder="Search notes…" value={query}
          onInput={(e) => setQuery((e.target as HTMLInputElement).value)} />
        <Search size={15} style={{ color: "var(--text-muted)", flexShrink: 0 }} />
      </div>

      <form class="toolbar" onSubmit={handleAdd}>
        <input type="text" placeholder="New note…" value={newText}
          onInput={(e) => setNewText((e.target as HTMLInputElement).value)} disabled={busy} />
        <button class="btn-primary" type="submit" disabled={busy || !newText.trim()}>
          <Plus size={14} /> Add
        </button>
      </form>

      {filtered.length === 0 && (
        <p class="empty-msg">{query ? "No matching notes." : "No notes yet — add one above."}</p>
      )}

      {view === "list" ? (
        <ul class="item-list">
          {filtered.map((note) => (
            <li key={note.id} class="item-row">
              <div class="item-row-header" onClick={() => expand(note)} style={{ cursor: "pointer" }}>
                <span style={{ color: "var(--text-muted)", flexShrink: 0 }}>
                  {expandedId === note.id ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                </span>
                <span class="item-name">
                  {note.loaded && note.content
                    ? note.content.slice(0, 80) + (note.content.length > 80 ? "…" : "")
                    : <span style={{ color: "var(--text-muted)", fontFamily: "monospace", fontSize: "0.78rem" }}>{note.id.slice(0, 8)}</span>}
                </span>
                <div class="item-actions" onClick={(e) => e.stopPropagation()}>
                  <button class="btn-icon" title="Edit" onClick={() => startEdit(note)}><Pencil size={13} /></button>
                  <button class="btn-icon btn-danger" title="Delete" onClick={() => handleDelete(note.id)}><Trash2 size={13} /></button>
                </div>
              </div>

              {expandedId === note.id && (
                <div class="note-body">
                  {editingId === note.id ? (
                    <div class="note-edit-form">
                      <textarea ref={editRef} class="note-editor" value={editValue} rows={6}
                        onInput={(e) => setEditValue((e.target as HTMLTextAreaElement).value)} />
                      <div class="btn-group">
                        <button class="btn-primary" onClick={() => saveEdit(note.id)}><Check size={13} /> Save</button>
                        <button onClick={() => setEditingId(null)}><X size={13} /> Cancel</button>
                      </div>
                    </div>
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
      ) : (
        <div class="card-grid">
          {filtered.map((note) => (
            <div key={note.id} class="note-card" onClick={() => expand(note)}>
              <div class="card-actions" onClick={(e) => e.stopPropagation()}>
                <button class="btn-icon" onClick={() => startEdit(note)}><Pencil size={13} /></button>
                <button class="btn-icon btn-danger" onClick={() => handleDelete(note.id)}><Trash2 size={13} /></button>
              </div>

              {editingId === note.id ? (
                <div class="note-edit-form" onClick={(e) => e.stopPropagation()}>
                  <textarea ref={editRef} class="note-editor" value={editValue} rows={4}
                    onInput={(e) => setEditValue((e.target as HTMLTextAreaElement).value)} />
                  <div class="btn-group">
                    <button class="btn-primary" onClick={() => saveEdit(note.id)}><Check size={13} /> Save</button>
                    <button onClick={() => setEditingId(null)}><X size={13} /></button>
                  </div>
                </div>
              ) : (
                <>
                  <div class="note-card-content">
                    {note.loaded && note.content
                      ? note.content.slice(0, 200) + (note.content.length > 200 ? "…" : "")
                      : <span class="note-id">{note.id.slice(0, 8)}…</span>}
                  </div>
                </>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
