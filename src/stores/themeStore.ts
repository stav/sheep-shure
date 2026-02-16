import { create } from "zustand";
import { tauriInvoke } from "@/lib/tauri";

type Theme = "light" | "dark" | "system";
type ResolvedTheme = "light" | "dark";

interface ThemeState {
  theme: Theme;
  resolvedTheme: ResolvedTheme;
  setTheme: (t: Theme) => void;
  initTheme: () => Promise<void>;
}

function getSystemTheme(): ResolvedTheme {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function applyTheme(resolved: ResolvedTheme) {
  document.documentElement.classList.toggle("dark", resolved === "dark");
}

function resolve(theme: Theme): ResolvedTheme {
  return theme === "system" ? getSystemTheme() : theme;
}

let mediaQueryCleanup: (() => void) | null = null;

export const useThemeStore = create<ThemeState>((set) => ({
  theme: "system",
  resolvedTheme: "light",

  setTheme: (t) => {
    const resolved = resolve(t);
    applyTheme(resolved);
    set({ theme: t, resolvedTheme: resolved });

    // Re-attach media query listener if switching to/from system
    setupMediaListener(t, set);

    // Persist to DB (fire-and-forget)
    tauriInvoke("update_settings", { settings: { theme: t } }).catch(() => {});
  },

  initTheme: async () => {
    let theme: Theme = "system";
    try {
      const settings = await tauriInvoke<Record<string, string>>("get_settings");
      const saved = settings?.theme;
      if (saved === "light" || saved === "dark" || saved === "system") {
        theme = saved;
      }
    } catch {
      // DB not ready or no settings — default to system
    }

    const resolved = resolve(theme);
    applyTheme(resolved);
    set({ theme, resolvedTheme: resolved });
    setupMediaListener(theme, set);
  },
}));

function setupMediaListener(
  theme: Theme,
  set: (partial: Partial<ThemeState>) => void
) {
  // Clean up previous listener
  if (mediaQueryCleanup) {
    mediaQueryCleanup();
    mediaQueryCleanup = null;
  }

  if (theme !== "system") return;

  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = (e: MediaQueryListEvent) => {
    const resolved: ResolvedTheme = e.matches ? "dark" : "light";
    applyTheme(resolved);
    set({ resolvedTheme: resolved });
  };
  mq.addEventListener("change", handler);
  mediaQueryCleanup = () => mq.removeEventListener("change", handler);
}
