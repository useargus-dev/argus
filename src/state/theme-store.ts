import { create } from "zustand";

import {
  getStoredTheme,
  setStoredTheme,
  toggleStoredTheme,
  type Theme,
} from "../lib/theme";

interface ThemeState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggle: () => void;
}

export const useThemeStore = create<ThemeState>((set) => ({
  theme: getStoredTheme(),
  setTheme: (theme) => {
    setStoredTheme(theme);
    set({ theme });
  },
  toggle: () => {
    const theme = toggleStoredTheme();
    set({ theme });
  },
}));
