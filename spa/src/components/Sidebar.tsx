import { useEffect, useRef, useState } from "preact/hooks";
import { Folder, Monitor, User, BarChart2, Share2, Plus, X } from "lucide-react";
import { fetchBoards, fetchSharedBoards, createBoard, type Board, type SharedBoard } from "../api";
import { sidebarVersion } from "../sidebarRefresh";

interface Props { activeRoute: string }

export function Sidebar({ activeRoute }: Props) {
  const [boards, setBoards] = useState<Board[]>([]);
  const [sharedBoards, setSharedBoards] = useState<SharedBoard[]>([]);
  const [showCreate, setShowCreate] = useState(false);
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const version = sidebarVersion.value;

  useEffect(() => { load(); }, [version]);
  useEffect(() => { if (showCreate) inputRef.current?.focus(); }, [showCreate]);

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

  const nav = [
    { href: "#/shared",  icon: <Share2 size={15} />,   label: "Shared",   key: "shared"  },
    { href: "#/devices", icon: <Monitor size={15} />,  label: "Devices",  key: "devices" },
    { href: "#/stats",   icon: <BarChart2 size={15} />, label: "Stats",   key: "stats"   },
    { href: "#/whoami",  icon: <User size={15} />,     label: "Profile",  key: "whoami"  },
  ];

  return (
    <>
      <div class="sidebar-section">
        <div class="sidebar-section-title">
          Boards
          <button class="btn-icon" onClick={() => setShowCreate(!showCreate)} title="New board">
            {showCreate ? <X size={13} /> : <Plus size={13} />}
          </button>
        </div>

        {showCreate && (
          <form class="sidebar-create-form" onSubmit={handleCreate}>
            <input
              ref={inputRef}
              type="text"
              placeholder="Board name…"
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
          return (
            <a key={b.id} class={`sidebar-item ${active ? "active" : ""}`} href={`#/board/${b.id}`}>
              <Folder size={14} />
              <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>{b.name}</span>
            </a>
          );
        })}

        {boards.length === 0 && !showCreate && (
          <div style={{ padding: "0.3rem 0.75rem", fontSize: "0.78rem", color: "var(--text-muted)" }}>
            No boards yet
          </div>
        )}
      </div>

      {sharedBoards.length > 0 && (
        <>
          <div class="sidebar-divider" />
          <div class="sidebar-section">
            <div class="sidebar-section-title">Shared boards</div>
            {sharedBoards.map((b) => {
              const active = activeRoute === `shared-board/${b.board_id}`;
              return (
                <a key={b.board_id} class={`sidebar-item ${active ? "active" : ""}`} href={`#/shared-board/${b.board_id}`}>
                  <Share2 size={14} />
                  <span style={{ overflow: "hidden", textOverflow: "ellipsis", flex: 1 }}>{b.board_name}</span>
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
