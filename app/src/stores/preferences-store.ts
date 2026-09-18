import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { Category } from "@/lib/foundry-filters";
import { ANY_REFINEMENT } from "@/lib/relic-filters";
import type { FissurePath, Timeframe } from "@/types";

interface PreferencesState {
  squadSize: number;
  refinement: string;
  era: string;
  fissurePath: FissurePath;
  timeframe: Timeframe;
  foundryCategory: Category;
  hiddenColumns: Record<string, string[]>;
  setSquadSize: (squadSize: number) => void;
  setRefinement: (refinement: string) => void;
  setEra: (era: string) => void;
  setFissurePath: (fissurePath: FissurePath) => void;
  setTimeframe: (timeframe: Timeframe) => void;
  setFoundryCategory: (foundryCategory: Category) => void;
  setHiddenColumns: (tableId: string, columns: string[]) => void;
}

const TABLE_KEY = /^merframe\.table\.(.+)\.hiddenColumns$/;

function legacy<T extends string>(key: string, fallback: T): T {
  return (localStorage.getItem(key) as T | null) ?? fallback;
}

function legacyHiddenColumns(): Record<string, string[]> {
  const hiddenColumns: Record<string, string[]> = {};
  for (const key of Object.keys(localStorage)) {
    const tableId = TABLE_KEY.exec(key)?.[1];
    if (tableId) {
      hiddenColumns[tableId] = JSON.parse(localStorage.getItem(key) ?? "[]");
    }
  }
  return hiddenColumns;
}

const legacySquadSize = localStorage.getItem("merframe.relicPlanner.squadSize");

export const usePreferencesStore = create<PreferencesState>()(
  persist(
    (set) => ({
      squadSize: legacySquadSize ? Number(legacySquadSize) : 4,
      refinement: legacy("merframe.relicPlanner.refinement", ANY_REFINEMENT),
      era: legacy("merframe.relicPlanner.era", "all"),
      fissurePath: legacy<FissurePath>("merframe.world.fissurePath", "normal"),
      timeframe: legacy<Timeframe>("merframe.stats.timeframe", "30"),
      foundryCategory: legacy<Category>(
        "merframe.foundry.category",
        "warframe",
      ),
      hiddenColumns: legacyHiddenColumns(),
      setSquadSize: (squadSize) => set({ squadSize }),
      setRefinement: (refinement) => set({ refinement }),
      setEra: (era) => set({ era }),
      setFissurePath: (fissurePath) => set({ fissurePath }),
      setTimeframe: (timeframe) => set({ timeframe }),
      setFoundryCategory: (foundryCategory) => set({ foundryCategory }),
      setHiddenColumns: (tableId, columns) =>
        set((state) => ({
          hiddenColumns: { ...state.hiddenColumns, [tableId]: columns },
        })),
    }),
    {
      name: "merframe.preferences",
      partialize: (state) => ({
        squadSize: state.squadSize,
        refinement: state.refinement,
        era: state.era,
        fissurePath: state.fissurePath,
        timeframe: state.timeframe,
        foundryCategory: state.foundryCategory,
        hiddenColumns: state.hiddenColumns,
      }),
    },
  ),
);
