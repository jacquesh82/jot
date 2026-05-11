import { ComponentChildren } from "preact";
import { Sun, Moon, BookOpen, LogOut } from "lucide-react";
import { theme, toggleTheme } from "../theme";
import { Sidebar } from "./Sidebar";
import { ToastContainer } from "./Toast";
import { t, localeSignal, setLocale, type Locale } from "../i18n";

interface Props {
  children: ComponentChildren;
  activeRoute: string;
}

function logout() {
  localStorage.removeItem("token");
  location.hash = "#/register";
}

const LOCALE_OPTIONS: { value: Locale; label: string }[] = [
  { value: "en", label: "🇺🇸 EN" },
  { value: "fr", label: "🇫🇷 FR" },
  { value: "es", label: "🇪🇸 ES" },
  { value: "de", label: "🇩🇪 DE" },
];

export function Layout({ children, activeRoute }: Props) {
  return (
    <div class="app-shell">
      <header class="app-header">
        <div class="app-logo">
          <BookOpen size={18} />
          jot
        </div>
        <div class="header-actions">
          <select
            value={localeSignal.value}
            onChange={(e) => setLocale((e.target as HTMLSelectElement).value as Locale)}
            title={t("layout.langSelect")}
            style={{ fontSize: "0.8rem", background: "var(--bg-surface)", color: "var(--text)", border: "1px solid var(--border)", borderRadius: "var(--radius)", padding: "0.2rem 0.35rem", cursor: "pointer" }}
          >
            {LOCALE_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>{o.label}</option>
            ))}
          </select>
          <button class="btn-ghost" onClick={toggleTheme} title={t("layout.toggleTheme")}>
            {theme.value === "light" ? <Moon size={16} /> : <Sun size={16} />}
          </button>
          <button class="btn-ghost" onClick={logout} title={t("layout.signOut")}>
            <LogOut size={16} />
          </button>
        </div>
      </header>

      <aside class="app-sidebar">
        <Sidebar activeRoute={activeRoute} />
      </aside>

      <main class="app-main">{children}</main>

      <ToastContainer />

      <footer class="app-footer">
        {t("layout.footer")}
        <a href="https://github.com/jacquesh82/jot" target="_blank" rel="noopener noreferrer" class="footer-link">GitHub</a>
      </footer>
    </div>
  );
}
