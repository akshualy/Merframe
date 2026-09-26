export interface StatPoint {
  snapshot: number;
  at: string;
  plat: number;
  credits: number;
  endo: number;
  ducats: number;
  aya: number | null;
  mr: number;
}

export interface TradeItem {
  name: string;
  count: number;
  rank: number | null;
}

export interface Trade {
  offered: TradeItem[];
  received: TradeItem[];
  plat: number;
}

export interface StoredTrade {
  id: number;
  at: string;
  partner: string | null;
  trade: Trade;
}

export interface RelicOpening {
  id: number;
  at: string;
  relic: string;
  reward_item: string;
  player_count: number;
}

export interface StoredDelta {
  item_type: string;
  category: string;
  delta: number;
}

export interface DailyCount {
  day: string;
  count: number;
}

export interface StatsSummary {
  account_created: string;
  snapshot_days: number;
  prime_owned: number;
  prime_total: number;
  prime_percent: number;
}

export interface StatsTab {
  series: StatPoint[];
  trades: StoredTrade[];
  relic_openings: RelicOpening[];
  latest_deltas: StoredDelta[];
  relics_per_day: DailyCount[];
  trades_per_day: DailyCount[];
  days_played: DailyCount[];
  summary: StatsSummary | null;
}
