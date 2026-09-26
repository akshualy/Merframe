import type { Update } from "@tauri-apps/plugin-updater";
import { create } from "zustand";
import type { GameStatus, Settings, WorldStateView } from "@/types";

interface AppState {
  ready: boolean;
  bootError: string | null;
  status: GameStatus | null;
  world: WorldStateView | null;
  settings: Settings | null;
  update: Update | null;
  setReady: (ready: boolean) => void;
  setBootError: (message: string | null) => void;
  setStatus: (status: GameStatus) => void;
  setWorld: (world: WorldStateView) => void;
  setSettings: (settings: Settings) => void;
  setUpdate: (update: Update | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  ready: false,
  bootError: null,
  status: null,
  world: null,
  settings: null,
  update: null,
  setReady: (ready) => set({ ready }),
  setBootError: (bootError) => set({ bootError }),
  setStatus: (status) => set({ status }),
  setWorld: (world) => set({ world }),
  setSettings: (settings) => set({ settings }),
  setUpdate: (update) => set({ update }),
}));
