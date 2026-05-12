import { useEffect, useRef, useState } from "preact/hooks";
import { Plus, Trash2, Search, LayoutList, LayoutGrid, X, Share2, UserPlus, Users, Check, Lock, LockOpen } from "lucide-react";
import { toast } from "../toast";
import {
  fetchNotes, createNote, deleteNote, connectWs,
  fetchBoardShares, shareBoardWith, revokeBoardShare,
  getRecentContacts,
  type Note, type WsEvent, type BoardShareEntry, type IdentityInfo,
} from "../api";
import { notesView } from "../viewMode";
import { selectedNoteId } from "../selectedNote";
import { refreshSidebar } from "../sidebarRefresh";
import { NoteEditor } from "./NoteEditor";
import { decryptBlock } from "../blocks/crypto";
import { t } from "../i18n";

interface Props { boardId: string; readOnly?: boolean }

export function NoteList({ boardId, readOnly = false }: Props) {
  const [notes, setNotes] = useState<Note[]>([]);
  const [titles, setTitles] = useState<Record<string, string>>({});
  const [newText, setNewText] = useState("");
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [showSharing, setShowSharing] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<string | null>(null);
  const [shares, setShares] = useState<BoardShareEntry[]>([]);
  const [shareTarget, setShareTarget] = useState("");
  const [shareError, setShareError] = useState<string | null>(null);
  const [recentContacts, setRecentContacts] = useState<IdentityInfo[]>([]);
  const stopWs = useRef<(() => void) | null>(null);
  const view = notesView.value;

  useEffect(() => {
    load();
    stopWs.current = connectWs(onWs);
    const onTitleChanged = () => load();
    window.addEventListener("note-title-changed", onTitleChanged);
    return () => {
      stopWs.current?.();
      window.removeEventListener("note-title-changed", onTitleChanged);
    };
  }, [boardId]);

  async function load() {
    try {
      const ns = await fetchNotes(boardId);
      setNotes(ns);
      const decoded: Record<string, string> = {};
      await Promise.all(ns.map(async n => {
        if (!n.title_b64) return;
        try { decoded[n.id] = await decryptBlock(boardId, n.id, n.title_b64); }
        catch { /* unreadable for this caller (e.g. legacy/shared) */ }
      }));
      setTitles(decoded);
    } catch (e) { setError(String(e)); }
  }

  function onWs(e: WsEvent) {
    // Reload on any note-level change so freshly-set titles surface here.
    if (typeof e.event === "string" && e.event.startsWith("note_")) load();
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

  async function loadShares() {
    try { setShares(await fetchBoardShares(boardId)); } catch {}
  }

  async function loadRecentContacts() {
    const contacts = await getRecentContacts();
    const sharedIds = new Set(shares.map(s => s.shared_with_id));
    setRecentContacts(contacts.filter(c => !sharedIds.has(c.id)));
  }

  async function handleShare(e: Event) {
    e.preventDefault();
    if (!shareTarget.trim()) return;
    setShareError(null);
    try {
      await shareBoardWith(boardId, shareTarget.trim());
      setShareTarget("");
      await loadShares();
      refreshSidebar();
    } catch (e) { setShareError(String(e)); }
  }

  async function handleRevoke(targetId: string) {
    try { await revokeBoardShare(boardId, targetId); await loadShares(); refreshSidebar(); } catch {}
  }

  function toggleSharing() {
    const next = !showSharing;
    setShowSharing(next);
    if (next) { loadShares(); loadRecentContacts(); }
  }

  function handleDeleteClick(e: Event, id: string) {
    e.stopPropagation();
    setPendingDelete(id);
  }

  async function handleDeleteConfirm(e: Event, id: string) {
    e.stopPropagation();
    setPendingDelete(null);
    try {
      await deleteNote(id);
      if (selectedNoteId.value === id) selectedNoteId.value = null;
      setNotes((p) => p.filter((n) => n.id !== id));
      toast(t("notelist.noteDeleted"), "success");
    } catch (err) { toast(String(err), "error"); }
  }

  function handleDeleteCancel(e: Event) {
    e.stopPropagation();
    setPendingDelete(null);
  }

  const filtered = query.trim()
    ? notes.filter((n) => {
        const q = query.toLowerCase();
        return n.id.includes(query)
          || (n.snippet ?? "").toLowerCase().includes(q)
          || (titles[n.id] ?? "").toLowerCase().includes(q);
      })
    : notes;

  const panelOpen = !!selectedNoteId.value;

  return (
    <div class={`notes-workspace ${panelOpen ? "panel-open" : ""}`}>
      <div class="notes-pane">
        <div class="page-title">
          <h2>{t("notelist.title")} {readOnly && <span style={{ fontSize: "0.72rem", color: "var(--text-muted)", fontWeight: 400 }}>{t("notelist.readOnly")}</span>}</h2>
          <div class="page-title-actions">
            {!readOnly && (
              <button class={`btn-icon ${showSharing ? "btn-primary" : ""}`} onClick={toggleSharing} title={t("notelist.shareBoard")}>
                <Share2 size={14} />
              </button>
            )}
            <div class="btn-group">
              <button class={`btn-icon ${view === "list" ? "btn-primary" : ""}`}
                onClick={() => (notesView.value = "list")} title={t("notelist.listView")}>
                <LayoutList size={15} />
              </button>
              <button class={`btn-icon ${view === "card" ? "btn-primary" : ""}`}
                onClick={() => (notesView.value = "card")} title={t("notelist.cardView")}>
                <LayoutGrid size={15} />
              </button>
            </div>
          </div>
        </div>

        {showSharing && (
          <div class="note-panel-sharing" style={{ margin: "0 0 0.75rem", borderRadius: "var(--radius)", border: "1px solid var(--border)" }}>
            <div class="sharing-title"><Share2 size={13} /> {t("notelist.shareTitle")}</div>
            {shares.length === 0
              ? <p class="sharing-empty">{t("notelist.notSharedYet")}</p>
              : (
                <ul class="sharing-list">
                  {shares.map((s) => (
                    <li key={s.shared_with_id} class="sharing-row">
                      <span class="sharing-name">{s.shared_with_name ?? s.shared_with_id.slice(0, 8)}</span>
                      <button class="btn-icon btn-danger" onClick={() => handleRevoke(s.shared_with_id)}><X size={12} /></button>
                    </li>
                  ))}
                </ul>
              )
            }
            {recentContacts.length > 0 && (
              <div class="sharing-contacts">
                {recentContacts.map((c) => (
                  <button key={c.id} class="contact-chip" type="button"
                    onClick={() => setShareTarget(c.friendly_name)}>
                    {c.friendly_name}
                  </button>
                ))}
              </div>
            )}
            <form class="sharing-form" onSubmit={handleShare}>
              <input type="text" placeholder={t("notelist.shareTargetPlaceholder")} value={shareTarget}
                onInput={(e) => setShareTarget((e.target as HTMLInputElement).value)} />
              <button class="btn-primary" type="submit" disabled={!shareTarget.trim()}><UserPlus size={13} /></button>
            </form>
            {shareError && <p class="sharing-error">{shareError}</p>}
          </div>
        )}

        {error && (
          <div class="error-msg">
            {error}
            <button class="btn-icon" onClick={() => setError(null)}><X size={14} /></button>
          </div>
        )}

        <div class="toolbar">
          <Search size={14} style={{ color: "var(--text-muted)", flexShrink: 0 }} />
          <input type="search" placeholder={t("notelist.searchPlaceholder")} value={query}
            onInput={(e) => setQuery((e.target as HTMLInputElement).value)} />
        </div>

        {!readOnly && (
          <form class="toolbar" onSubmit={handleAdd}>
            <input type="text" placeholder={t("notelist.newNotePlaceholder")} value={newText}
              onInput={(e) => setNewText((e.target as HTMLInputElement).value)} disabled={busy} />
            <button class="btn-primary" type="submit" disabled={busy || !newText.trim()}>
              <Plus size={14} /> {t("notelist.add")}
            </button>
          </form>
        )}

        {filtered.length === 0 && (
          <p class="empty-msg">{query ? t("notelist.noMatchingNotes") : t("notelist.noNotes")}</p>
        )}

        {view === "list" ? (
          <ul class="item-list">
            {filtered.map((note) => {
              const active = selectedNoteId.value === note.id;
              const isPending = pendingDelete === note.id;
              return (
                <li key={note.id} class={`item-row ${active ? "note-active" : ""}`}
                  onClick={() => (selectedNoteId.value = note.id)} style={{ cursor: "pointer" }}>
                  <div class="item-row-header">
                    <span class="item-name">
                      {titles[note.id] || note.snippet || <span style={{ fontFamily: "monospace", opacity: 0.5 }}>{note.id.slice(0, 8)}</span>}
                    </span>
                    <span class="lock-tip" data-tip={note.encrypted ? t("notelist.encrypted") : t("notelist.notEncrypted")}>
                      {note.encrypted
                        ? <Lock size={12} style={{ color: "var(--success)", flexShrink: 0 }} />
                        : <LockOpen size={12} style={{ color: "#ff9f0a", flexShrink: 0 }} />
                      }
                    </span>
                    {note.shared && <Users size={12} style={{ color: "var(--accent)", flexShrink: 0 }} title={t("notelist.shared")} />}
                    {!readOnly && (
                      <div class="item-actions" onClick={(e) => e.stopPropagation()}>
                        {isPending ? (
                          <>
                            <button class="btn-icon btn-danger" title={t("notelist.confirmDelete")} onClick={(e) => handleDeleteConfirm(e, note.id)}><Check size={13} /></button>
                            <button class="btn-icon" title={t("notelist.cancelDelete")} onClick={handleDeleteCancel}><X size={13} /></button>
                          </>
                        ) : (
                          <button class="btn-icon btn-danger" title={t("notelist.deleteNote")} onClick={(e) => handleDeleteClick(e, note.id)}><Trash2 size={13} /></button>
                        )}
                      </div>
                    )}
                  </div>
                </li>
              );
            })}
          </ul>
        ) : (
          <div class="card-grid">
            {filtered.map((note) => {
              const active = selectedNoteId.value === note.id;
              const isPending = pendingDelete === note.id;
              return (
                <div key={note.id} class={`note-card ${active ? "note-card-active" : ""}`}
                  onClick={() => (selectedNoteId.value = note.id)}>
                  {!readOnly && (
                    <div class="card-actions" onClick={(e) => e.stopPropagation()}>
                      {isPending ? (
                        <>
                          <button class="btn-icon btn-danger" title={t("notelist.confirmDelete")} onClick={(e) => handleDeleteConfirm(e, note.id)}><Check size={13} /></button>
                          <button class="btn-icon" title={t("notelist.cancelDelete")} onClick={handleDeleteCancel}><X size={13} /></button>
                        </>
                      ) : (
                        <button class="btn-icon btn-danger" title={t("notelist.deleteNote")} onClick={(e) => handleDeleteClick(e, note.id)}><Trash2 size={13} /></button>
                      )}
                    </div>
                  )}
                  <span class="lock-tip note-card-lock" data-tip={note.encrypted ? t("notelist.encrypted") : t("notelist.notEncrypted")}>
                    {note.encrypted
                      ? <Lock size={11} style={{ color: "var(--success)" }} />
                      : <LockOpen size={11} style={{ color: "#ff9f0a" }} />
                    }
                  </span>
                  {note.shared && <Users size={11} class="note-card-shared-badge" title={t("notelist.shared")} />}
                  <span class="note-card-snippet">
                    {titles[note.id] || note.snippet || <span class="note-id">{note.id.slice(0, 8)}…</span>}
                  </span>
                  <span class="note-card-type">{note.note_type}</span>
                </div>
              );
            })}
          </div>
        )}
      </div>

      <NoteEditor
        onDeleted={(id) => setNotes((p) => p.filter((n) => n.id !== id))}
        onEncrypted={() => load()}
      />
    </div>
  );
}
