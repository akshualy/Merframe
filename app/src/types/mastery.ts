export type MasteryGroup = "warframes" | "weapons" | "companions" | "other";

export type MasteryOrdering = "closest" | "from_relics" | "by_platinum";

export interface MasteryComponent {
  unique_name: string;
  name: string;
  image_name: string | null;
  owned: number;
  required: number;
  enough: boolean;
}

export interface MasteryItem {
  unique_name: string;
  name: string;
  kind: string;
  group: MasteryGroup;
  image_name: string | null;
  owned: boolean;
  mastered: boolean;
  level: Level;
  acquisition: Acquisition;
  favourite: boolean;
  components: MasteryComponent[];
}

export interface Level {
  current: number;
  max: number;
  xp_remaining: number;
}

export interface Acquisition {
  missing_parts: number;
  plat_cost: number;
  purchasable: boolean;
  relic_probability: number;
}

export interface CategoryTotals {
  current: number;
  max: number;
  percent: number;
}

export interface MasterySummary {
  warframes: CategoryTotals;
  weapons: CategoryTotals;
  companions: CategoryTotals;
  star_normal: CategoryTotals;
  star_steel: CategoryTotals;
  star_junctions: CategoryTotals;
  star_steel_junctions: CategoryTotals;
  intrinsic_railjack: CategoryTotals;
  intrinsic_duviri: CategoryTotals;
  content_percent: number;
  star_percent: number;
  intrinsic_percent: number;
}

export interface RouteMember {
  name: string;
  detail: string;
  xp: number;
}

export interface LevelUpRoute {
  kind: string;
  label: string;
  unit: string;
  count: number;
  xp_available: number;
  members: RouteMember[];
}

export interface MasteryTab {
  rank: number;
  founder: boolean;
  include_founders: boolean;
  percent: number;
  rank_xp_earned: number;
  rank_xp_span: number;
  summary: MasterySummary;
  plat_total: number;
  favourite_xp: number;
  favourite_percent: number;
  recommended: MasteryItem[];
  routes: LevelUpRoute[];
}
