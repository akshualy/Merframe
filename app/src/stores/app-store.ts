import type { Update } from "@tauri-apps/plugin-updater";
import { toast } from "sonner";
import { create } from "zustand";
import { api, errorMessage } from "@/lib/bridge";
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
  saveSettings: (patch: Partial<Settings>) => Promise<string | null>;
  setUpdate: (update: Update | null) => void;
}

function started(settings: Settings | null): Settings {
  if (settings === null) {
    throw new Error("Settings are read before the app finished starting");
  }

  return settings;
}

export const useAppStore = create<AppState>((set, get) => ({
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
  saveSettings: async (patch) => {
    const previous = started(get().settings);
    const settings = { ...previous, ...patch };
    set({ settings });

    try {
      await api.settingsSet(settings);
      return null;
    } catch (error) {
      set({ settings: previous });
      const message = errorMessage(error);
      toast.error(message);
      return message;
    }
  },
  setUpdate: (update) => set({ update }),
}));

export function useSettings(): Settings {
  return started(useAppStore((state) => state.settings));
}
