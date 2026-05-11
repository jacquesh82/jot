import { useEffect, useRef, useState } from "preact/hooks";
import { X, Save, Share2, Trash2, Eye, Code, UserPlus, Check, Lock } from "lucide-react";
import { toast } from "../toast";
import { t } from "../i18n";

const PANEL_WIDTH_KEY = "jot:panel-width";
const DEFAULT_WIDTH = 480;
const MIN_WIDTH = 260;
const MAX_WIDTH = 1200;
import { marked } from "marked";
import DOMPurify from "dompurify";
import {
  fetchNoteContent, updateNoteContent, deleteNote, encryptExistingNote,
  fetchShares, shareNote, revokeShare,
  type ShareEntry,
} from "../api";
import { selectedNoteId } from "../selectedNote";

interface Props {
  onDeleted: (id: string) => void;
  onEncrypted?: (id: string) => void;
}

type ViewMode = "raw" | "preview";

export function NoteEditor({ onDeleted, onEncrypted }: Props) {
  const noteId = selectedNoteId.value;
  const [content, setContent] = useState("");
  const [draft, setDraft] = useState("");
  const [mode, setMode] = useState<ViewMode>("raw");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [shares, setShares] = useState<ShareEntry[]>([]);
  const [shareTarget, setShareTarget] = useState("");
  const [sharePermission, setSharePermission] = useState<"read" | "write" | "delete">("read");
  const [shareError, setShareError] = useState<string | null>(null);
  const [showSharing, setShowSharing] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [isEncrypted, setIsEncrypted] = useState(true);
  const [encrypting, setEncrypting] = useState(false);
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
      const { content: text, encrypted } = await fetchNoteContent(id);
      setContent(text);
      setDraft(text);
      setIsEncrypted(encrypted);
    } catch (e) {
      setLoadError(String(e));
    }
  }

  async function handleEncrypt() {
    if (!noteId) return;
    setEncrypting(true);
    try {
      await encryptExistingNote(noteId, draft);
      setIsEncrypted(true);
      toast(t("editor.noteEncrypted"), "success");
      onEncrypted?.(noteId);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setEncrypting(false);
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
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    if (!noteId) return;
    if (!confirmDelete) { setConfirmDelete(true); return; }
    setConfirmDelete(false);
    try {
      await deleteNote(noteId);
      toast(t("editor.noteDeleted"), "success");
      selectedNoteId.value = null;
      onDeleted(noteId);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function handleShare(e: Event) {
    e.preventDefault();
    if (!noteId || !shareTarget.trim()) return;
    setShareError(null);
    try {
      await shareNote(noteId, shareTarget.trim(), sharePermission);
      setShareTarget("");
      setSharePermission("read");
      await loadShares(noteId);
    } catch (e) {
      setShareError(String(e));
    }
  }

  async function handleRevoke(targetId: string) {
    if (!noteId) return;
    try {
      await revokeShare(noteId, targetId);
      await loadShares(noteId);
    } catch (e) {
      toast(String(e), "error");
    }
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
                onClick={() => setMode("raw")} title={t("editor.rawText")}
              >
                <Code size={14} />
              </button>
              <button
                class={`btn-icon ${mode === "preview" ? "btn-primary" : ""}`}
                onClick={() => setMode("preview")} title={t("editor.markdownPreview")}
              >
                <Eye size={14} />
              </button>
            </div>
            <div class="btn-group" style={{ marginLeft: "auto" }}>
              {!isEncrypted && (
                <button class="btn-icon lock-tip" data-tip={t("editor.encryptNote")} onClick={handleEncrypt} disabled={encrypting} style={{ color: "#ff9f0a" }}>
                  <Lock size={14} />
                </button>
              )}
              <button class="btn-icon" onClick={toggleSharing} title={t("editor.sharing")}>
                <Share2 size={14} />
              </button>
              {confirmDelete ? (
                <>
                  <button class="btn-icon btn-danger" onClick={handleDelete} title={t("editor.confirmDelete")}>
                    <Check size={14} />
                  </button>
                  <button class="btn-icon" onClick={() => setConfirmDelete(false)} title={t("editor.cancelDelete")}>
                    <X size={14} />
                  </button>
                </>
              ) : (
                <button class="btn-icon btn-danger" onClick={handleDelete} title={t("editor.deleteNote")}>
                  <Trash2 size={14} />
                </button>
              )}
              <button class="btn-icon" onClick={() => (selectedNoteId.value = null)} title={t("editor.close")}>
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
                placeholder={t("editor.notePlaceholder")}
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
                <Save size={13} /> {saving ? t("editor.saving") : t("editor.save")}
              </button>
              <button onClick={() => { setDraft(content); setDirty(false); }}>
                <X size={13} /> {t("editor.discard")}
              </button>
            </div>
          )}

          {/* ── Sharing section ── */}
          {showSharing && (
            <div class="note-panel-sharing">
              <div class="sharing-title">
                <Share2 size={13} /> {t("editor.sharedWith")}
              </div>
              {shares.length === 0 ? (
                <p class="sharing-empty">{t("editor.notSharedYet")}</p>
              ) : (
                <ul class="sharing-list">
                  {shares.map((s) => (
                    <li key={s.shared_with_id} class="sharing-row">
                      <span class="sharing-name">
                        {s.shared_with_name ?? s.shared_with_id.slice(0, 8)}
                      </span>
                      <span class={`perm-badge perm-${s.permission ?? "read"}`}>
                        {s.permission ?? "read"}
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
                  placeholder={t("editor.shareTargetPlaceholder")}
                  value={shareTarget}
                  onInput={(e) => setShareTarget((e.target as HTMLInputElement).value)}
                />
                <select
                  class="perm-select"
                  value={sharePermission}
                  onChange={(e) => setSharePermission((e.target as HTMLSelectElement).value as "read" | "write" | "delete")}
                >
                  <option value="read">read</option>
                  <option value="write">read+write</option>
                  <option value="delete">read+write+delete</option>
                </select>
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
