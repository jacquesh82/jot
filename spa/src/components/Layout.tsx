import { ComponentChildren } from "preact";
import { Sun, Moon, BookOpen, LogOut } from "lucide-react";
import { theme, toggleTheme } from "../theme";
import { Sidebar } from "./Sidebar";

interface Props {
  children: ComponentChildren;
  activeRoute: string;
}

function logout() {
  localStorage.removeItem("token");
  location.hash = "#/register";
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
          <button class="btn-ghost" onClick={logout} title="Sign out">
            <LogOut size={16} />
          </button>
        </div>
      </header>

      <aside class="app-sidebar">
        <Sidebar activeRoute={activeRoute} />
      </aside>

      <main class="app-main">{children}</main>

      <footer class="app-footer">
        jot · encrypted notes · v0.1.0
        <a href="https://github.com/jacquesh82/jot" target="_blank" rel="noopener noreferrer" class="footer-link">GitHub</a>
      </footer>
    </div>
  );
}
