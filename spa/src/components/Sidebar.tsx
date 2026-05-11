import { useEffect, useRef, useState } from "preact/hooks";
import { Folder, Monitor, User, BarChart2, Share2, Plus, X, Pencil, Trash2, Check } from "lucide-react";
import { fetchBoards, fetchSharedBoards, createBoard, renameBoard, deleteBoard, type Board, type SharedBoard } from "../api";
import { sidebarVersion, refreshSidebar } from "../sidebarRefresh";
import { t } from "../i18n";

interface Props { activeRoute: string }

export function Sidebar({ activeRoute }: Props) {
  const [boards, setBoards] = useState<Board[]>([]);
  const [sharedBoards, setSharedBoards] = useState<SharedBoard[]>([]);
  const [showCreate, setShowCreate] = useState(false);
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [pendingDelete, setPendingDelete] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const editRef = useRef<HTMLInputElement>(null);
  const version = sidebarVersion.value;

  useEffect(() => { load(); }, [version]);
  useEffect(() => {
    window.addEventListener("hashchange", load);
    window.addEventListener("focus", load);
    return () => {
      window.removeEventListener("hashchange", load);
      window.removeEventListener("focus", load);
    };
  }, []);
  useEffect(() => { if (showCreate) inputRef.current?.focus(); }, [showCreate]);
  useEffect(() => { if (editingId) editRef.current?.focus(); }, [editingId]);

  async function load() {
    try { setBoards(await fetchBoards()); } catch {}
    try { setSharedBoards(await fetchSharedBoards()); } catch {}
  }

  async function handleCreate(e: Event) {
    e.preventDefault();
    if (!newName.trim()) return;
    setCreating(true);
    try {
      await createBoard(newName.trim());
      setNewName("");
      setShowCreate(false);
      await load();
    } finally {
      setCreating(false);
    }
  }

  function startEdit(e: Event, board: Board) {
    e.preventDefault();
    e.stopPropagation();
    setPendingDelete(null);
    setEditingId(board.id);
    setEditName(board.name);
  }

  async function commitRename(e: Event) {
    e.preventDefault();
    if (!editingId || !editName.trim()) { setEditingId(null); return; }
    try {
      await renameBoard(editingId, editName.trim());
      setEditingId(null);
      await load();
    } catch { setEditingId(null); }
  }

  function cancelEdit(e: Event) {
    e.stopPropagation();
    setEditingId(null);
  }

  function askDelete(e: Event, id: string) {
    e.preventDefault();
    e.stopPropagation();
    setEditingId(null);
    setPendingDelete(id);
  }

  async function confirmDelete(e: Event, id: string) {
    e.preventDefault();
    e.stopPropagation();
    setPendingDelete(null);
    try {
      await deleteBoard(id);
      if (location.hash.includes(id)) location.hash = "#/";
      refreshSidebar();
    } catch {}
  }

  function cancelDelete(e: Event) {
    e.stopPropagation();
    setPendingDelete(null);
  }

  const nav = [
    { href: "#/shared",  icon: <Share2 size={15} />,   label: t("sidebar.shared"),   key: "shared"  },
    { href: "#/devices", icon: <Monitor size={15} />,  label: t("sidebar.devices"),  key: "devices" },
    { href: "#/stats",   icon: <BarChart2 size={15} />, label: t("sidebar.stats"),   key: "stats"   },
    { href: "#/whoami",  icon: <User size={15} />,     label: t("sidebar.profile"),  key: "whoami"  },
  ];

  return (
    <>
      <div class="sidebar-section">
        <div class="sidebar-section-title">
          {t("sidebar.boards")}
          <button class="btn-icon" onClick={() => setShowCreate(!showCreate)} title={t("sidebar.newBoard")}>
            {showCreate ? <X size={13} /> : <Plus size={13} />}
          </button>
        </div>

        {showCreate && (
          <form class="sidebar-create-form" onSubmit={handleCreate}>
            <input
              ref={inputRef}
              type="text"
              placeholder={t("sidebar.boardNamePlaceholder")}
              value={newName}
              onInput={(e) => setNewName((e.target as HTMLInputElement).value)}
              disabled={creating}
            />
            <button class="btn-primary" type="submit" disabled={creating || !newName.trim()}>
              <Plus size={12} />
            </button>
          </form>
        )}

        {boards.map((b) => {
          const active = activeRoute === `board/${b.id}`;
          const isEditing = editingId === b.id;
          const isPending = pendingDelete === b.id;

          if (isEditing) {
            return (
              <form key={b.id} class="sidebar-create-form" onSubmit={commitRename}
                style={{ margin: "1px 0.4rem" }}>
                <input
                  ref={editRef}
                  type="text"
                  value={editName}
                  onInput={(e) => setEditName((e.target as HTMLInputElement).value)}
                  onKeyDown={(e) => { if (e.key === "Escape") cancelEdit(e); }}
                />
                <button class="btn-icon" type="submit" title={t("sidebar.save")} disabled={!editName.trim()}>
                  <Check size={12} />
                </button>
                <button class="btn-icon" type="button" title={t("sidebar.cancel")} onClick={cancelEdit}>
                  <X size={12} />
                </button>
              </form>
            );
          }

          if (isPending) {
            return (
              <div key={b.id} class={`sidebar-item ${active ? "active" : ""}`}
                style={{ gap: "0.4rem" }}>
                <Trash2 size={14} style={{ color: "var(--danger)", flexShrink: 0 }} />
                <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", fontSize: "0.8rem" }}>
                  {t("sidebar.deleteConfirm", { name: b.name })}
                </span>
                <button class="btn-icon btn-danger" title={t("sidebar.confirmDelete")} onClick={(e) => confirmDelete(e, b.id)}>
                  <Check size={12} />
                </button>
                <button class="btn-icon" title={t("sidebar.cancel")} onClick={cancelDelete}>
                  <X size={12} />
                </button>
              </div>
            );
          }

          return (
            <a key={b.id} class={`sidebar-item sidebar-board-item ${active ? "active" : ""}`}
              href={`#/board/${b.id}`}>
              <Folder size={14} style={{ flexShrink: 0 }} />
              <span style={{ overflow: "hidden", textOverflow: "ellipsis", flex: 1 }}>{b.name}</span>
              <span class="sidebar-board-actions">
                <button class="btn-icon" title={t("sidebar.rename")} onClick={(e) => startEdit(e, b)}>
                  <Pencil size={11} />
                </button>
                <button class="btn-icon btn-danger" title={t("sidebar.delete")} onClick={(e) => askDelete(e, b.id)}>
                  <Trash2 size={11} />
                </button>
              </span>
            </a>
          );
        })}

        {boards.length === 0 && !showCreate && (
          <div style={{ padding: "0.3rem 0.75rem", fontSize: "0.78rem", color: "var(--text-muted)" }}>
            {t("sidebar.noBoards")}
          </div>
        )}
      </div>

      {sharedBoards.length > 0 && (
        <>
          <div class="sidebar-divider" />
          <div class="sidebar-section">
            <div class="sidebar-section-title">{t("sidebar.sharedBoards")}</div>
            {sharedBoards.map((b) => {
              const active = activeRoute === `shared-board/${b.board_id}`;
              return (
                <a key={b.board_id} class={`sidebar-item sidebar-item-2line ${active ? "active" : ""}`} href={`#/shared-board/${b.board_id}`}>
                  <Share2 size={14} style={{ flexShrink: 0, marginTop: "2px" }} />
                  <span style={{ overflow: "hidden", flex: 1, minWidth: 0 }}>
                    <span style={{ display: "block", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{b.board_name}</span>
                    <span style={{ display: "block", fontSize: "0.7rem", color: "var(--text-muted)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {b.owner_friendly_name ?? b.owner_identity_id.slice(0, 8)}
                    </span>
                  </span>
                </a>
              );
            })}
          </div>
        </>
      )}

      <div class="sidebar-divider" />

      <div class="sidebar-section">
        {nav.map(({ href, icon, label, key }) => (
          <a key={key} class={`sidebar-item ${activeRoute === key ? "active" : ""}`} href={href}>
            {icon}
            {label}
          </a>
        ))}
      </div>
    </>
  );
}
