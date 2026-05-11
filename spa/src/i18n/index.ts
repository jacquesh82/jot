import { signal } from "@preact/signals";
import { en } from "./en";
import { fr } from "./fr";
import { es } from "./es";
import { de } from "./de";

export type Locale = "en" | "fr" | "es" | "de";

const VALID: Locale[] = ["en", "fr", "es", "de"];

function detect(): Locale {
  const stored = localStorage.getItem("jot:locale");
  if (stored && VALID.includes(stored as Locale)) return stored as Locale;
  const browser = navigator.language.slice(0, 2) as Locale;
  if (VALID.includes(browser)) return browser;
  return "en";
}

export const localeSignal = signal<Locale>(detect());

export function setLocale(l: Locale): void {
  localeSignal.value = l;
  localStorage.setItem("jot:locale", l);
}

const locales: Record<Locale, Record<string, string>> = { en, fr, es, de };

export function t(key: string, vars?: Record<string, string | number>): string {
  const map = locales[localeSignal.value] ?? locales.en;
  let s = (map[key] ?? locales.en[key]) ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.replace(`{${k}}`, String(v));
    }
  }
  return s;
}
