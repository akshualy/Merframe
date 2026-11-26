export type FissurePath = "baseline" | "normal" | "hard" | "storm" | "all";
export type ArcanesTabView = "collection" | "packs" | "inventory";
export type Timeframe = "today" | "7" | "30" | "90" | "all";
export interface StatsHistoryItem {
  itemType: string;
  colorSlot: number;
}
export type TimeframeCard =
  | "history"
  | "items"
  | "changes"
  | "relics"
  | "trades"
  | "tradeHistory";
