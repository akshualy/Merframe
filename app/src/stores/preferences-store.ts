import { create } from "zustand";
import { persist } from "zustand/middleware";
import { ANY } from "@/lib/filters";
import type { Category } from "@/lib/foundry-filters";
import { readStored, readStoredJson } from "@/lib/storage";
import type {
  FissurePath,
  ResourceScope,
  ResourceSource,
  Timeframe,
  TimeframeCard,
} from "@/types";

interface PreferencesState {
  squadSize: number;
  refinement: string;
  era: string;
  fissurePath: FissurePath;
  timeframes: Record<TimeframeCard, Timeframe>;
  foundryCategory: Category;
  resourceSource: ResourceSource;
  resourceScope: ResourceScope;
  hiddenColumns: Record<string, string[]>;
  pageSize: number;
  gridPageSize: number;
  setSquadSize: (squadSize: number) => void;
  setRefinement: (refinement: string) => void;
  setEra: (era: string) => void;
  setFissurePath: (fissurePath: FissurePath) => void;
  setTimeframe: (card: TimeframeCard, timeframe: Timeframe) => void;
  setFoundryCategory: (foundryCategory: Category) => void;
  setResourceSource: (resourceSource: ResourceSource) => void;
  setResourceScope: (resourceScope: ResourceScope) => void;
  setHiddenColumns: (tableId: string, columns: string[]) => void;
  setPageSize: (pageSize: number) => void;
  setGridPageSize: (gridPageSize: number) => void;
}

const TABLE_KEY = /^merframe\.table\.(.+)\.hiddenColumns$/;

function legacy<T extends string>(key: string, fallback: T): T {
  return (readStored(key) as T | null) ?? fallback;
}

function legacyHiddenColumns(): Record<string, string[]> {
  const hiddenColumns: Record<string, string[]> = {};
  for (const key of Object.keys(localStorage)) {
    const tableId = TABLE_KEY.exec(key)?.[1];
    if (tableId) {
      hiddenColumns[tableId] = readStoredJson<string[]>(key) ?? [];
    }
  }
  return hiddenColumns;
}

export const DEFAULT_SQUAD_SIZE = 4;

const DEFAULT_TIMEFRAMES: Record<TimeframeCard, Timeframe> = {
  history: "7",
  changes: "today",
  relics: "today",
  trades: "today",
  tradeHistory: "30",
};

const legacySquadSize = readStored("merframe.relicPlanner.squadSize");

export const usePreferencesStore = create<PreferencesState>()(
  persist(
    (set) => ({
      squadSize: legacySquadSize ? Number(legacySquadSize) : DEFAULT_SQUAD_SIZE,
      refinement: legacy("merframe.relicPlanner.refinement", ANY),
      era: legacy("merframe.relicPlanner.era", "all"),
      fissurePath: legacy<FissurePath>("merframe.world.fissurePath", "normal"),
      timeframes: DEFAULT_TIMEFRAMES,
      foundryCategory: legacy<Category>(
        "merframe.foundry.category",
        "warframe",
      ),
      resourceSource: "held",
      resourceScope: "mastery",
      hiddenColumns: legacyHiddenColumns(),
      pageSize: 50,
      gridPageSize: 48,
      setSquadSize: (squadSize) => set({ squadSize }),
      setRefinement: (refinement) => set({ refinement }),
      setEra: (era) => set({ era }),
      setFissurePath: (fissurePath) => set({ fissurePath }),
      setTimeframe: (card, timeframe) =>
        set((state) => ({
          timeframes: { ...state.timeframes, [card]: timeframe },
        })),
      setFoundryCategory: (foundryCategory) => set({ foundryCategory }),
      setResourceSource: (resourceSource) => set({ resourceSource }),
      setResourceScope: (resourceScope) => set({ resourceScope }),
      setHiddenColumns: (tableId, columns) =>
        set((state) => ({
          hiddenColumns: { ...state.hiddenColumns, [tableId]: columns },
        })),
      setPageSize: (pageSize) => set({ pageSize }),
      setGridPageSize: (gridPageSize) => set({ gridPageSize }),
    }),
    {
      name: "merframe.preferences",
      partialize: (state) => ({
        squadSize: state.squadSize,
        refinement: state.refinement,
        era: state.era,
        fissurePath: state.fissurePath,
        timeframes: state.timeframes,
        foundryCategory: state.foundryCategory,
        resourceSource: state.resourceSource,
        resourceScope: state.resourceScope,
        hiddenColumns: state.hiddenColumns,
        pageSize: state.pageSize,
        gridPageSize: state.gridPageSize,
      }),
      merge: (persisted, current) => {
        const stored = persisted as Partial<PreferencesState>;
        return {
          ...current,
          ...stored,
          timeframes: { ...current.timeframes, ...stored.timeframes },
        };
      },
    },
  ),
);
