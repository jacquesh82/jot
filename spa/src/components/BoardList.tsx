import { useEffect, useState } from "preact/hooks";
import { fetchBoards, type Board } from "../api";

export function BoardList() {
  const [boards, setBoards] = useState<Board[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchBoards()
      .then(setBoards)
      .catch((e) => setError(String(e)));
  }, []);

  if (error) return <div class="error">Failed to load boards: {error}</div>;

  return (
    <div class="board-list">
      <h2>Boards</h2>
      {boards.length === 0 ? (
        <p class="empty">No boards — run <code>jot new board "My Board"</code></p>
      ) : (
        <ul>
          {boards.map((b) => (
            <li key={b.id}>
              <a href={`#/board/${b.id}`}>{b.name}</a>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
