import { useEffect, useRef, useState } from "preact/hooks";
import { Monitor, Pencil, Trash2, X, Check } from "lucide-react";
import { fetchDevices, renameDevice, deleteDevice, decodeJwt, type DeviceSummary } from "../api";

export function DevicesPage() {
  const [devices, setDevices] = useState<DeviceSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameVal, setRenameVal] = useState("");
  const renameRef = useRef<HTMLInputElement>(null);
  const currentDeviceId = decodeJwt()?.sub ?? "";

  useEffect(() => { load(); }, []);
  useEffect(() => { if (renamingId) renameRef.current?.focus(); }, [renamingId]);

  async function load() {
    try { setDevices(await fetchDevices()); }
    catch (e) { setError(String(e)); }
  }

  function startRename(d: DeviceSummary) { setRenamingId(d.id); setRenameVal(d.name); }

  async function commitRename(id: string) {
    if (!renameVal.trim()) { setRenamingId(null); return; }
    try { await renameDevice(id, renameVal.trim()); setRenamingId(null); await load(); }
    catch (e) { setError(String(e)); }
  }

  async function handleDelete(id: string, name: string) {
    if (!confirm(`Delete device "${name}"?`)) return;
    try { await deleteDevice(id); await load(); }
    catch (e) { setError(String(e)); }
  }

  function fmtDate(iso: string) {
    try { return new Date(iso).toLocaleString(); } catch { return iso; }
  }

  return (
    <div>
      <div class="page-title"><h2>Devices</h2></div>

      {error && (
        <div class="error-msg">
          {error}
          <button class="btn-icon" onClick={() => setError(null)}><X size={14} /></button>
        </div>
      )}

      {devices.length === 0 ? (
        <p class="empty-msg">No devices found.</p>
      ) : (
        <ul class="item-list">
          {devices.map((d) => {
            const isCurrent = d.id === currentDeviceId;
            return (
              <li key={d.id} class="item-row">
                <div class="item-row-header">
                  <Monitor size={15} style={{ color: "var(--text-muted)", flexShrink: 0 }} />

                  {renamingId === d.id ? (
                    <input ref={renameRef} class="rename-input" value={renameVal}
                      onInput={(e) => setRenameVal((e.target as HTMLInputElement).value)}
                      onBlur={() => commitRename(d.id)}
                      onKeyDown={(e) => { if (e.key === "Enter") commitRename(d.id); if (e.key === "Escape") setRenamingId(null); }} />
                  ) : (
                    <div style={{ flex: 1, overflow: "hidden" }}>
                      <div style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}>
                        <span class="item-name">{d.name}</span>
                        {isCurrent && <span class="device-badge current">current</span>}
                      </div>
                      <div class="device-meta">{d.id.slice(0, 8)} · last seen {fmtDate(d.last_seen)}</div>
                    </div>
                  )}

                  <div class="item-actions">
                    <button class="btn-icon" title="Rename" onClick={() => startRename(d)}><Pencil size={13} /></button>
                    {!isCurrent && (
                      <button class="btn-icon btn-danger" title="Delete" onClick={() => handleDelete(d.id, d.name)}>
                        <Trash2 size={13} />
                      </button>
                    )}
                  </div>
                </div>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
