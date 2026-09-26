import type { ArcaneRarity } from "@/components/game-icon";

export type VaultStatus = "vaulted" | "available" | "unknown";

export interface Prices {
  sell: number | null;
  buy: number | null;
  ducats: number | null;
}

export interface ItemStatus {
  built: boolean;
  mastered: boolean;
}

export interface PartSet {
  name: string;
  complete: boolean;
}

export interface PartRow {
  name: string;
  unique_name: string;
  image_name: string | null;
  count: number;
  prices: Prices;
  set: PartSet;
  vault: VaultStatus | null;
  item: ItemStatus;
  prime: boolean;
  market_slug: string;
  favourite: boolean;
  order_placed: boolean;
}

export interface UpgradePrices {
  sell: number | null;
  sell_max_rank: number | null;
  is_floor: boolean;
  buy: number | null;
}

export interface ModHolder {
  item_id: string;
  name: string;
  custom_name: string | null;
  image_name: string | null;
  rank: number | null;
  takes_orokin_reactor: boolean;
  orokin_upgrade: boolean;
  exilus_adapter: boolean;
  configs: number[];
  forma: number;
  archon_shards: number;
}

export interface ModRow {
  name: string;
  unique_name: string;
  image_name: string | null;
  count: number;
  rank: number | null;
  max_rank: number | null;
  prices: UpgradePrices;
  rarity: ArcaneRarity | null;
  prime: boolean;
  equipped_in: ModHolder[];
  market_slug: string;
  favourite: boolean;
  order_placed: boolean;
}

export interface RelicRow {
  relic: string;
  tier: string;
  refinement: string;
  image_name: string | null;
  count: number;
  vault: VaultStatus;
  unique_name: string;
  plat: number | null;
  favourite: boolean;
  order_placed: boolean;
}

export interface MiscRow {
  name: string;
  unique_name: string;
  image_name: string | null;
  count: number;
  ducats: number | null;
  plat: number | null;
  market_slug: string;
  favourite: boolean;
  order_placed: boolean;
}

export interface SetComponent {
  unique_name: string;
  name: string;
  image_name: string | null;
  owned: number;
  required: number;
  enough: boolean;
}

export interface SetRow {
  set_name: string;
  unique_name: string;
  image_name: string | null;
  owned_parts: number;
  total_parts: number;
  count: number;
  complete: boolean;
  item: ItemStatus;
  vault: VaultStatus | null;
  prices: Prices;
  market_slug: string;
  favourite: boolean;
  order_placed: boolean;
  components: SetComponent[];
}

export interface TabTotals {
  ducats: number;
  plat: number;
}

export interface InventoryTab {
  parts: PartRow[];
  mods: ModRow[];
  arcanes: ModRow[];
  relics: RelicRow[];
  misc: MiscRow[];
  sets: SetRow[];
  totals: Record<string, TabTotals>;
}

export type ResourceSource = "held" | "craftable";
export type ResourceScope = "mastery" | "all" | "starred";

export interface ResourceUse {
  unique_name: string;
  name: string;
  image_name: string | null;
  wiki_url: string | null;
  amount: number;
  favourite: boolean;
  ready_to_build: boolean;
}

export interface ResourceRow {
  unique_name: string;
  name: string;
  image_name: string | null;
  owned: number;
  required: number;
  deficit: number;
  used_by: ResourceUse[];
}

export interface ResourcesTab {
  resources: ResourceRow[];
  items: number;
  credits: number;
}
