import { useEffect, useState } from "preact/hooks";
import { BarChart2, Folder, FileText, Monitor, RefreshCw } from "lucide-react";
import { fetchBoards, fetchNotes, fetchDevices, type Board } from "../api";

interface BoardStat { board: Board; count: number }

export function StatsPage() {
  const [boardStats, setBoardStats] = useState<BoardStat[]>([]);
  const [deviceCount, setDeviceCount] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => { load(); }, []);

  async function load() {
    setLoading(true);
    setError(null);
    try {
      const [boards, devices] = await Promise.all([fetchBoards(), fetchDevices()]);
      setDeviceCount(devices.length);
      const stats = await Promise.all(
        boards.map(async (board) => {
          try {
            const notes = await fetchNotes(board.id);
            return { board, count: notes.length };
          } catch {
            return { board, count: 0 };
          }
        })
      );
      setBoardStats(stats);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  const totalNotes = boardStats.reduce((s, b) => s + b.count, 0);
  const maxCount = Math.max(...boardStats.map((b) => b.count), 1);

  return (
    <div>
      <div class="page-title">
        <h2>Stats</h2>
        <div class="page-title-actions">
          <button class="btn-icon" onClick={load} disabled={loading} title="Refresh">
            <RefreshCw size={14} style={loading ? { animation: "spin 1s linear infinite" } : {}} />
          </button>
        </div>
      </div>

      {error && <div class="error-msg">{error}</div>}

      {loading ? (
        <p class="empty-msg">Loading…</p>
      ) : (
        <>
          <div class="stats-grid">
            <div class="stat-card">
              <Folder size={18} style={{ color: "var(--accent)" }} />
              <div class="stat-value">{boardStats.length}</div>
              <div class="stat-label">Boards</div>
            </div>
            <div class="stat-card">
              <FileText size={18} style={{ color: "var(--accent)" }} />
              <div class="stat-value">{totalNotes}</div>
              <div class="stat-label">Notes</div>
            </div>
            <div class="stat-card">
              <Monitor size={18} style={{ color: "var(--accent)" }} />
              <div class="stat-value">{deviceCount}</div>
              <div class="stat-label">Devices</div>
            </div>
            {boardStats.length > 0 && (
              <div class="stat-card">
                <BarChart2 size={18} style={{ color: "var(--accent)" }} />
                <div class="stat-value">{Math.round(totalNotes / boardStats.length * 10) / 10}</div>
                <div class="stat-label">Avg / board</div>
              </div>
            )}
          </div>

          {boardStats.length > 0 && (
            <>
              <h3 style={{ marginBottom: "0.75rem", fontSize: "0.95rem", fontWeight: 600 }}>Notes per board</h3>
              <div class="bar-chart">
                {boardStats
                  .slice()
                  .sort((a, b) => b.count - a.count)
                  .map(({ board, count }) => (
                    <div key={board.id} class="bar-row">
                      <a class="bar-label" href={`#/board/${board.id}`} title={board.name}>{board.name}</a>
                      <div class="bar-track">
                        <div class="bar-fill" style={{ width: `${(count / maxCount) * 100}%` }} />
                      </div>
                      <span class="bar-count">{count}</span>
                    </div>
                  ))}
              </div>
            </>
          )}
        </>
      )}

      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}
