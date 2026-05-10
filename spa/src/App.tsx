import { useEffect, useState } from "preact/hooks";
import { Layout } from "./components/Layout";
import { NoteList } from "./components/NoteList";
import { DevicesPage } from "./components/DevicesPage";
import { WhoamiPage } from "./components/WhoamiPage";
import { StatsPage } from "./components/StatsPage";
import { SharedNotesPage } from "./components/SharedNotesPage";
import { DeviceRegister } from "./components/DeviceRegister";
import { Folder } from "lucide-react";

type RouteView = "home" | "board" | "devices" | "stats" | "whoami" | "shared" | "register";
interface Route { view: RouteView; boardId?: string }

function parseHash(): Route {
  const h = location.hash.replace(/^#\/?/, "");
  if (!h || h === "") return { view: "home" };
  if (h === "register") return { view: "register" };
  if (h === "devices")  return { view: "devices" };
  if (h === "stats")    return { view: "stats" };
  if (h === "whoami")   return { view: "whoami" };
  if (h === "shared")   return { view: "shared" };
  if (h.startsWith("board/")) return { view: "board", boardId: h.slice(6) };
  return { view: "home" };
}

function activeRouteKey(route: Route): string {
  if (route.view === "board" && route.boardId) return `board/${route.boardId}`;
  return route.view;
}

export function App() {
  const [route, setRoute] = useState<Route>(parseHash);

  useEffect(() => {
    const onChange = () => setRoute(parseHash());
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);

  useEffect(() => {
    if (!localStorage.getItem("token") && route.view !== "register") {
      location.hash = "#/register";
    }
  }, [route]);

  if (route.view === "register" || !localStorage.getItem("token")) {
    return <DeviceRegister />;
  }

  return (
    <Layout activeRoute={activeRouteKey(route)}>
      {route.view === "board" && route.boardId ? (
        <NoteList boardId={route.boardId} />
      ) : route.view === "devices" ? (
        <DevicesPage />
      ) : route.view === "stats" ? (
        <StatsPage />
      ) : route.view === "whoami" ? (
        <WhoamiPage />
      ) : route.view === "shared" ? (
        <SharedNotesPage />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", height: "60%", gap: "0.75rem", color: "var(--text-muted)" }}>
          <Folder size={40} strokeWidth={1} />
          <p style={{ fontSize: "0.9rem" }}>Select a board from the sidebar</p>
        </div>
      )}
    </Layout>
  );
}
