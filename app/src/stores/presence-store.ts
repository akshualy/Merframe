import { create } from "zustand";
import type { Presence } from "@/types";

interface PresenceState {
  presence: Presence;
  setPresence: (presence: Presence) => void;
}

export const usePresenceStore = create<PresenceState>((set) => ({
  presence: { status: null, auto: false, live: null },
  setPresence: (presence) => set({ presence }),
}));
