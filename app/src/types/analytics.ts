import type { StoredTrade } from "@/types/stats";

export type MarketWindow = "2" | "7" | "30" | "90";

export type TradeCategory =
  | "set"
  | "prime"
  | "riven"
  | "arcane"
  | "relic"
  | "mod"
  | "other";

export interface MarketMover {
  slug: string;
  name: string;
  image_name: string | null;
  category: TradeCategory;
  unit_price: number;
  volume: number;
  value: number;
  price_change: number | null;
  volume_change: number | null;
}

export interface CategoryStatement {
  category: TradeCategory;
  revenue: number;
  expenses: number;
}

export interface TradedTotal {
  name: string;
  image_name: string | null;
  amount: number;
  value: number;
}

export interface PartnerTotal {
  name: string;
  sales: number;
  sales_value: number;
  purchases: number;
  purchases_value: number;
}

export interface TradeAnalytics {
  categories: CategoryStatement[];
  sold: TradedTotal[];
  bought: TradedTotal[];
  trades: StoredTrade[];
}
