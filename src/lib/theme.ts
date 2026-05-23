export type Theme = "light" | "dark";

const STORAGE_KEY = "argus-theme";

export function getStoredTheme(): Theme {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "light" || v === "dark") return v;
  } catch {
    /* private mode */
  }
  return "light";
}

export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  root.classList.toggle("dark", theme === "dark");
  root.dataset.theme = theme;
}

export function initTheme() {
  applyTheme(getStoredTheme());
}

export function setStoredTheme(theme: Theme) {
  try {
    localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    /* ignore */
  }
  applyTheme(theme);
}

export function toggleStoredTheme(): Theme {
  const next: Theme = getStoredTheme() === "light" ? "dark" : "light";
  setStoredTheme(next);
  return next;
}
