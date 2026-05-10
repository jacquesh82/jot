import { useEffect, useState } from "preact/hooks";
import { BoardList } from "./components/BoardList";
import { NoteList } from "./components/NoteList";
import { DeviceRegister } from "./components/DeviceRegister";

function parseHash(): { view: string; boardId?: string } {
  const hash = location.hash.replace(/^#\/?/, "");
  if (hash.startsWith("board/")) return { view: "board", boardId: hash.slice(6) };
  if (hash === "register") return { view: "register" };
  return { view: "boards" };
}

export function App() {
  const [route, setRoute] = useState(parseHash);

  useEffect(() => {
    const onHashChange = () => setRoute(parseHash());
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  useEffect(() => {
    if (!localStorage.getItem("token") && route.view !== "register") {
      location.hash = "#/register";
    }
  }, [route]);

  if (route.view === "register") return <DeviceRegister />;
  if (route.view === "board" && route.boardId) return <NoteList boardId={route.boardId} />;
  return <BoardList />;
}
