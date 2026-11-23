import { create } from "zustand";
import type { PlatformFamily, ViewId } from "@/types/catalog";

type ThemeMode = "dark" | "light";

interface WorkspaceState {
  view: ViewId;
  selectedGameId: string | null;
  selectedFeatureId: string | null;
  platform: "all" | PlatformFamily;
  search: string;
  theme: ThemeMode;
  inspectorOpen: boolean;
  selectView: (view: ViewId) => void;
  selectGame: (gameId: string) => void;
  selectFeature: (featureId: string) => void;
  filterPlatform: (platform: "all" | PlatformFamily) => void;
  setSearch: (search: string) => void;
  setTheme: (mode: ThemeMode) => void;
  toggleInspector: () => void;
}

export const useWorkspace = create<WorkspaceState>((set) => ({
  view: "library",
  selectedGameId: null,
  selectedFeatureId: null,
  platform: "all",
  search: "",
  theme: "dark",
  inspectorOpen: false,
  selectView: (view) => set({ view, selectedFeatureId: null }),
  selectGame: (selectedGameId) => set({
    view: "workspace", selectedGameId, selectedFeatureId: null, search: "",
  }),
  selectFeature: (selectedFeatureId) => set({ view: "workspace", selectedFeatureId }),
  filterPlatform: (platform) => set({
    platform, view: "library", selectedFeatureId: null,
  }),
  setSearch: (search) => set({ search, view: "library" }),
  setTheme: (theme) => set({ theme }),
  toggleInspector: () => set((previous) => ({ inspectorOpen: !previous.inspectorOpen })),
}));
