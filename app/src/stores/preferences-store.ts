import { create } from "zustand";
import { persist } from "zustand/middleware";
import { ANY } from "@/lib/filters";
import type { Category } from "@/lib/foundry-filters";
import { readStored, readStoredJson } from "@/lib/storage";
import type {
  ArcanesTabView,
  FissurePath,
  ResourceScope,
  ResourceSource,
  StatsHistoryItem,
  Timeframe,
  TimeframeCard,
} from "@/types";

interface PreferencesState {
  squadSize: number;
  refinement: string;
  era: string;
  fissurePath: FissurePath;
  timeframes: Record<TimeframeCard, Timeframe>;
  historyItems: StatsHistoryItem[];
  foundryCategory: Category;
  resourceSource: ResourceSource;
  resourceScope: ResourceScope;
  arcaneView: ArcanesTabView;
  marketTotalsWithHiddenOrders: boolean;
  hiddenColumns: Record<string, string[]>;
  pageSize: number;
  gridPageSize: number;
  setSquadSize: (squadSize: number) => void;
  setRefinement: (refinement: string) => void;
  setEra: (era: string) => void;
  setFissurePath: (fissurePath: FissurePath) => void;
  setTimeframe: (card: TimeframeCard, timeframe: Timeframe) => void;
  setHistoryItems: (historyItems: StatsHistoryItem[]) => void;
  setFoundryCategory: (foundryCategory: Category) => void;
  setResourceSource: (resourceSource: ResourceSource) => void;
  setResourceScope: (resourceScope: ResourceScope) => void;
  setArcaneView: (arcaneView: ArcanesTabView) => void;
  setMarketTotalsWithHiddenOrders: (
    marketTotalsWithHiddenOrders: boolean,
  ) => void;
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
  items: "30",
  changes: "today",
  relics: "today",
  trades: "today",
  tradeHistory: "30",
};

const DEFAULT_HISTORY_ITEMS: StatsHistoryItem[] = [
  { itemType: "/Lotus/Types/Items/MiscItems/PrimeBucks", colorSlot: 1 },
  { itemType: "/Lotus/Types/Items/MiscItems/SchismKey", colorSlot: 2 },
  { itemType: "/Lotus/Types/Items/MiscItems/Kuva", colorSlot: 3 },
];

const legacySquadSize = readStored("merframe.relicPlanner.squadSize");

export const usePreferencesStore = create<PreferencesState>()(
  persist(
    (set) => ({
      squadSize: legacySquadSize ? Number(legacySquadSize) : DEFAULT_SQUAD_SIZE,
      refinement: legacy("merframe.relicPlanner.refinement", ANY),
      era: legacy("merframe.relicPlanner.era", "all"),
      fissurePath: legacy<FissurePath>(
        "merframe.world.fissurePath",
        "baseline",
      ),
      timeframes: DEFAULT_TIMEFRAMES,
      historyItems: DEFAULT_HISTORY_ITEMS,
      foundryCategory: legacy<Category>(
        "merframe.foundry.category",
        "warframe",
      ),
      resourceSource: "held",
      resourceScope: "mastery",
      arcaneView: "inventory",
      marketTotalsWithHiddenOrders: false,
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
      setHistoryItems: (historyItems) => set({ historyItems }),
      setFoundryCategory: (foundryCategory) => set({ foundryCategory }),
      setResourceSource: (resourceSource) => set({ resourceSource }),
      setResourceScope: (resourceScope) => set({ resourceScope }),
      setArcaneView: (arcaneView) => set({ arcaneView }),
      setMarketTotalsWithHiddenOrders: (marketTotalsWithHiddenOrders) =>
        set({ marketTotalsWithHiddenOrders }),
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
        historyItems: state.historyItems,
        foundryCategory: state.foundryCategory,
        resourceSource: state.resourceSource,
        resourceScope: state.resourceScope,
        arcaneView: state.arcaneView,
        marketTotalsWithHiddenOrders: state.marketTotalsWithHiddenOrders,
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
