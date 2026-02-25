import { create } from "zustand";

interface AppState {
  sidebarCollapsed: boolean;
  pageSubtitle: string | null;
  toggleSidebar: () => void;
  setSidebarCollapsed: (value: boolean) => void;
  setPageSubtitle: (subtitle: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  sidebarCollapsed: false,
  pageSubtitle: null,
  toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
  setSidebarCollapsed: (value) => set({ sidebarCollapsed: value }),
  setPageSubtitle: (subtitle) => set({ pageSubtitle: subtitle }),
}));
