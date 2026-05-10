import { useEffect, useRef, useState } from "preact/hooks";
import { X, Save, Share2, Trash2, Eye, Code, UserPlus } from "lucide-react";

const PANEL_WIDTH_KEY = "jot:panel-width";
const DEFAULT_WIDTH = 480;
const MIN_WIDTH = 260;
const MAX_WIDTH = 1200;
import { marked } from "marked";
import DOMPurify from "dompurify";
import {
  fetchNoteContent, updateNoteContent, deleteNote,
  fetchShares, shareNote, revokeShare,
  type ShareEntry,
} from "../api";
import { selectedNoteId } from "../selectedNote";

interface Props {
  onDeleted: (id: string) => void;
}

type ViewMode = "raw" | "preview";

export function NoteEditor({ onDeleted }: Props) {
  const noteId = selectedNoteId.value;
  const [content, setContent] = useState("");
  const [draft, setDraft] = useState("");
  const [mode, setMode] = useState<ViewMode>("raw");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [shares, setShares] = useState<ShareEntry[]>([]);
  const [shareTarget, setShareTarget] = useState("");
  const [shareError, setShareError] = useState<string | null>(null);
  const [showSharing, setShowSharing] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [panelWidth, setPanelWidth] = useState(() => {
    const v = localStorage.getItem(PANEL_WIDTH_KEY);
    return v ? parseInt(v, 10) : DEFAULT_WIDTH;
  });
  const [resizing, setResizing] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (!noteId) return;
    setContent("");
    setDraft("");
    setDirty(false);
    setLoadError(null);
    setShowSharing(false);
    setShareError(null);
    load(noteId);
  }, [noteId]);

  useEffect(() => {
    if (mode === "raw" && textareaRef.current) textareaRef.current.focus();
  }, [mode]);

  async function load(id: string) {
    try {
      const text = await fetchNoteContent(id);
      setContent(text);
      setDraft(text);
    } catch (e) {
      setLoadError(String(e));
    }
  }

  async function loadShares(id: string) {
    try { setShares(await fetchShares(id)); } catch {}
  }

  async function save() {
    if (!noteId || !dirty) return;
    setSaving(true);
    try {
      await updateNoteContent(noteId, draft);
      setContent(draft);
      setDirty(false);
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    if (!noteId || !confirm("Delete this note?")) return;
    await deleteNote(noteId);
    selectedNoteId.value = null;
    onDeleted(noteId);
  }

  async function handleShare(e: Event) {
    e.preventDefault();
    if (!noteId || !shareTarget.trim()) return;
    setShareError(null);
    try {
      await shareNote(noteId, shareTarget.trim());
      setShareTarget("");
      await loadShares(noteId);
    } catch (e) {
      setShareError(String(e));
    }
  }

  async function handleRevoke(targetId: string) {
    if (!noteId) return;
    await revokeShare(noteId, targetId);
    await loadShares(noteId);
  }

  function toggleSharing() {
    const next = !showSharing;
    setShowSharing(next);
    if (next && noteId) loadShares(noteId);
  }

  function startResize(e: MouseEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = panelWidth;
    let currentWidth = startWidth;
    setResizing(true);

    function onMove(ev: MouseEvent) {
      currentWidth = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, startWidth + (startX - ev.clientX)));
      setPanelWidth(currentWidth);
    }
    function onUp() {
      setResizing(false);
      localStorage.setItem(PANEL_WIDTH_KEY, String(currentWidth));
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    }
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  }

  const html = DOMPurify.sanitize(marked.parse(draft) as string);
  const open = !!noteId;

  return (
    <aside
      class={`note-panel ${open ? "open" : ""} ${resizing ? "resizing" : ""}`}
      style={open ? { width: `${panelWidth}px` } : undefined}
    >
      {open && (
        <>
          <div class="note-panel-resize-handle" onMouseDown={startResize} />
          {/* ── Header ── */}
          <div class="note-panel-header">
            <span class="note-panel-id">{noteId?.slice(0, 8)}</span>
            <div class="btn-group">
              <button
                class={`btn-icon ${mode === "raw" ? "btn-primary" : ""}`}
                onClick={() => setMode("raw")} title="Raw text"
              >
                <Code size={14} />
              </button>
              <button
                class={`btn-icon ${mode === "preview" ? "btn-primary" : ""}`}
                onClick={() => setMode("preview")} title="Markdown preview"
              >
                <Eye size={14} />
              </button>
            </div>
            <div class="btn-group" style={{ marginLeft: "auto" }}>
              <button class="btn-icon" onClick={toggleSharing} title="Sharing">
                <Share2 size={14} />
              </button>
              <button class="btn-icon btn-danger" onClick={handleDelete} title="Delete note">
                <Trash2 size={14} />
              </button>
              <button class="btn-icon" onClick={() => (selectedNoteId.value = null)} title="Close">
                <X size={14} />
              </button>
            </div>
          </div>

          {/* ── Error ── */}
          {loadError && <div class="error-msg" style={{ margin: "0.5rem" }}>{loadError}</div>}

          {/* ── Body ── */}
          <div class="note-panel-body">
            {mode === "raw" ? (
              <textarea
                ref={textareaRef}
                class="note-panel-textarea"
                value={draft}
                onInput={(e) => {
                  setDraft((e.target as HTMLTextAreaElement).value);
                  setDirty(true);
                }}
                placeholder="Write your note…"
                spellcheck={false}
              />
            ) : (
              <div
                class="note-panel-preview"
                dangerouslySetInnerHTML={{ __html: html }}
              />
            )}
          </div>

          {/* ── Save bar ── */}
          {dirty && (
            <div class="note-panel-footer">
              <button class="btn-primary" onClick={save} disabled={saving}>
                <Save size={13} /> {saving ? "Saving…" : "Save"}
              </button>
              <button onClick={() => { setDraft(content); setDirty(false); }}>
                <X size={13} /> Discard
              </button>
            </div>
          )}

          {/* ── Sharing section ── */}
          {showSharing && (
            <div class="note-panel-sharing">
              <div class="sharing-title">
                <Share2 size={13} /> Shared with
              </div>
              {shares.length === 0 ? (
                <p class="sharing-empty">Not shared yet.</p>
              ) : (
                <ul class="sharing-list">
                  {shares.map((s) => (
                    <li key={s.shared_with_id} class="sharing-row">
                      <span class="sharing-name">
                        {s.shared_with_name ?? s.shared_with_id.slice(0, 8)}
                      </span>
                      <button class="btn-icon btn-danger" onClick={() => handleRevoke(s.shared_with_id)}>
                        <X size={12} />
                      </button>
                    </li>
                  ))}
                </ul>
              )}
              <form class="sharing-form" onSubmit={handleShare}>
                <input
                  type="text"
                  placeholder="Friendly name or UUID…"
                  value={shareTarget}
                  onInput={(e) => setShareTarget((e.target as HTMLInputElement).value)}
                />
                <button class="btn-primary" type="submit" disabled={!shareTarget.trim()}>
                  <UserPlus size={13} />
                </button>
              </form>
              {shareError && <p class="sharing-error">{shareError}</p>}
            </div>
          )}
        </>
      )}
    </aside>
  );
}
