import { useEffect, useRef, useState } from "preact/hooks";
import {
  X, Save, Share2, Trash2, Eye, Code, UserPlus, Check, Lock,
  Bold, Italic, Strikethrough, Heading1, Heading2, Heading3,
  Code2, Quote, List, ListOrdered, Link2, Minus,
} from "lucide-react";
import { toast } from "../toast";
import { t } from "../i18n";

const PANEL_WIDTH_KEY = "jot:panel-width";
const DEFAULT_WIDTH = 480;
const MIN_WIDTH = 260;
const MAX_WIDTH = 1200;
import { marked } from "marked";
import DOMPurify from "dompurify";
import {
  fetchNoteContent, fetchNoteMeta, updateNoteContent, deleteNote, encryptExistingNote,
  fetchShares, shareNote, revokeShare,
  type ShareEntry,
} from "../api";
import { selectedNoteId } from "../selectedNote";
import { BlockEditor } from "./BlockEditor";

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
  const [boardId, setBoardId] = useState<string | null>(null);
  const [schemaVersion, setSchemaVersion] = useState<number>(0);
  const [noteType, setNoteType] = useState<string>("text");
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
      const meta = await fetchNoteMeta(id);
      setBoardId(meta.board_id);
      setSchemaVersion(meta.schema_version ?? 0);
      setNoteType(meta.note_type);
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

  // ── Markdown insertion helpers ───────────────────────────────────────────────
  function insert(before: string, after: string, placeholder: string) {
    const el = textareaRef.current;
    if (!el) return;
    const s = el.selectionStart;
    const e = el.selectionEnd;
    const sel = draft.slice(s, e);
    const inner = sel || placeholder;
    const next = draft.slice(0, s) + before + inner + after + draft.slice(e);
    setDraft(next);
    setDirty(true);
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(s + before.length, s + before.length + inner.length);
    });
  }

  function insertLink() {
    const el = textareaRef.current;
    if (!el) return;
    const s = el.selectionStart;
    const e = el.selectionEnd;
    const sel = draft.slice(s, e);
    const label = sel || "text";
    const next = draft.slice(0, s) + `[${label}](url)` + draft.slice(e);
    setDraft(next);
    setDirty(true);
    requestAnimationFrame(() => {
      el.focus();
      const urlStart = s + 1 + label.length + 2;
      el.setSelectionRange(urlStart, urlStart + 3);
    });
  }

  function insertCodeBlock() {
    const el = textareaRef.current;
    if (!el) return;
    const s = el.selectionStart;
    const e = el.selectionEnd;
    const sel = draft.slice(s, e);
    const inner = sel || "code";
    const block = "```\n" + inner + "\n```";
    const next = draft.slice(0, s) + block + draft.slice(e);
    setDraft(next);
    setDirty(true);
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(s + 4, s + 4 + inner.length);
    });
  }

  function insertHR() {
    const el = textareaRef.current;
    if (!el) return;
    const s = el.selectionStart;
    const hr = "\n---\n";
    const next = draft.slice(0, s) + hr + draft.slice(s);
    setDraft(next);
    setDirty(true);
    requestAnimationFrame(() => {
      el.focus();
      const pos = s + hr.length;
      el.setSelectionRange(pos, pos);
    });
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey) {
      switch (e.key) {
        case "b": e.preventDefault(); insert("**", "**", "bold"); break;
        case "i": e.preventDefault(); insert("*", "*", "italic"); break;
        case "k": e.preventDefault(); insertLink(); break;
      }
    }
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

          {/* ── Markdown toolbar (raw mode only) ── */}
          {mode === "raw" && (
            <div class="note-panel-toolbar">
              <button class="tb-btn" title="Bold (Ctrl+B)" onClick={() => insert("**", "**", "bold")}><Bold size={13} /></button>
              <button class="tb-btn" title="Italic (Ctrl+I)" onClick={() => insert("*", "*", "italic")}><Italic size={13} /></button>
              <button class="tb-btn" title="Strikethrough" onClick={() => insert("~~", "~~", "text")}><Strikethrough size={13} /></button>
              <span class="tb-sep" />
              <button class="tb-btn" title="Heading 1" onClick={() => insert("# ", "", "Heading")}><Heading1 size={13} /></button>
              <button class="tb-btn" title="Heading 2" onClick={() => insert("## ", "", "Heading")}><Heading2 size={13} /></button>
              <button class="tb-btn" title="Heading 3" onClick={() => insert("### ", "", "Heading")}><Heading3 size={13} /></button>
              <span class="tb-sep" />
              <button class="tb-btn" title="Inline code" onClick={() => insert("`", "`", "code")}><Code size={13} /></button>
              <button class="tb-btn" title="Code block" onClick={insertCodeBlock}><Code2 size={13} /></button>
              <span class="tb-sep" />
              <button class="tb-btn" title="Blockquote" onClick={() => insert("> ", "", "quote")}><Quote size={13} /></button>
              <button class="tb-btn" title="Bulleted list" onClick={() => insert("- ", "", "item")}><List size={13} /></button>
              <button class="tb-btn" title="Numbered list" onClick={() => insert("1. ", "", "item")}><ListOrdered size={13} /></button>
              <span class="tb-sep" />
              <button class="tb-btn" title="Link (Ctrl+K)" onClick={insertLink}><Link2 size={13} /></button>
              <button class="tb-btn" title="Horizontal rule" onClick={insertHR}><Minus size={13} /></button>
            </div>
          )}

          {/* ── Error ── */}
          {loadError && <div class="error-msg" style={{ margin: "0.5rem" }}>{loadError}</div>}

          {/* ── Body ── */}
          <div class="note-panel-body">
            {noteType === "text" && schemaVersion >= 1 && boardId && noteId ? (
              <BlockEditor noteId={noteId} boardId={boardId} />
            ) : mode === "raw" ? (
              <textarea
                ref={textareaRef}
                class="note-panel-textarea"
                value={draft}
                onInput={(e) => {
                  setDraft((e.target as HTMLTextAreaElement).value);
                  setDirty(true);
                }}
                onKeyDown={handleKeyDown}
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
