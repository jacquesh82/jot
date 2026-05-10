import { ComponentChildren } from "preact";
import { Sun, Moon, BookOpen } from "lucide-react";
import { theme, toggleTheme } from "../theme";
import { Sidebar } from "./Sidebar";

interface Props {
  children: ComponentChildren;
  activeRoute: string;
}

export function Layout({ children, activeRoute }: Props) {
  return (
    <div class="app-shell">
      <header class="app-header">
        <div class="app-logo">
          <BookOpen size={18} />
          jot
        </div>
        <div class="header-actions">
          <button class="btn-ghost" onClick={toggleTheme} title="Toggle theme">
            {theme.value === "light" ? <Moon size={16} /> : <Sun size={16} />}
          </button>
        </div>
      </header>

      <aside class="app-sidebar">
        <Sidebar activeRoute={activeRoute} />
      </aside>

      <main class="app-main">{children}</main>

      <footer class="app-footer">jot · encrypted notes · v0.1.0</footer>
    </div>
  );
}
