import { useEffect, useState } from "preact/hooks";
import { Share2, RefreshCw } from "lucide-react";
import { getSharedWithMe, fetchNoteContent, type SharedNote } from "../api";
import { selectedNoteId } from "../selectedNote";
import { NoteEditor } from "./NoteEditor";
import { marked } from "marked";
import DOMPurify from "dompurify";
import { t } from "../i18n";

interface SharedNoteWithContent extends SharedNote { content?: string; loaded?: boolean }

export function SharedNotesPage() {
  const [notes, setNotes] = useState<SharedNoteWithContent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const panelOpen = !!selectedNoteId.value;

  useEffect(() => { load(); }, []);

  async function load() {
    setLoading(true);
    try { setNotes((await getSharedWithMe()).map((n) => ({ ...n }))); }
    catch (e) { setError(String(e)); }
    finally { setLoading(false); }
  }

  async function expand(note: SharedNoteWithContent) {
    selectedNoteId.value = note.note_id;
    if (!note.loaded) {
      const { content } = await fetchNoteContent(note.note_id);
      setNotes((p) =>
        p.map((n) => n.note_id === note.note_id ? { ...n, content, loaded: true } : n)
      );
    }
  }

  return (
    <div class={`notes-workspace ${panelOpen ? "panel-open" : ""}`}>
      <div class="notes-pane">
        <div class="page-title">
          <h2>{t("shared.title")}</h2>
          <div class="page-title-actions">
            <button class="btn-icon" onClick={load} disabled={loading} title={t("shared.refresh")}>
              <RefreshCw size={14} style={loading ? { animation: "spin 1s linear infinite" } : {}} />
            </button>
          </div>
        </div>

        {error && <div class="error-msg">{error}</div>}

        {loading ? (
          <p class="empty-msg">{t("shared.loading")}</p>
        ) : notes.length === 0 ? (
          <p class="empty-msg">{t("shared.noNotes")}</p>
        ) : (
          <ul class="item-list">
            {notes.map((note) => {
              const active = selectedNoteId.value === note.note_id;
              const html = note.loaded ? DOMPurify.sanitize(marked.parse(note.content ?? "") as string) : null;
              return (
                <li key={note.note_id} class={`item-row ${active ? "note-active" : ""}`}
                  onClick={() => expand(note)} style={{ cursor: "pointer" }}>
                  <div class="item-row-header">
                    <Share2 size={13} style={{ color: "var(--text-muted)", flexShrink: 0 }} />
                    <div style={{ flex: 1, overflow: "hidden" }}>
                      <div class="item-name" style={{ fontFamily: "monospace", fontSize: "0.8rem" }}>
                        {note.snippet || note.note_id.slice(0, 8)}
                      </div>
                      <div style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>
                        {t("shared.from", { owner: note.owner_friendly_name ?? note.owner_identity_id.slice(0, 8) })}
                      </div>
                    </div>
                  </div>
                  {note.loaded && html && (
                    <div class="note-body">
                      <div class="note-panel-preview"
                        dangerouslySetInnerHTML={{ __html: html }}
                        style={{ maxHeight: "120px", overflow: "hidden" }}
                      />
                    </div>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </div>

      <NoteEditor onDeleted={() => load()} />
      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}
