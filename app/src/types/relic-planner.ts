export interface RewardBreakdown {
  unique_name: string;
  name: string;
  image_name: string | null;
  rarity: string;
  plat: number | null;
  ducats: number | null;
  ownership: RewardOwnership;
  forma: boolean;
  favourite: boolean;
}

export interface RewardOwnership {
  owned: number;
  needed: number;
  needed_for_set: boolean;
  parent_owned: boolean;
}

export interface RefinementValue {
  refinement: string;
  expected_plat: number;
  expected_ducats: number;
  wanted_chance: number;
  traces: number;
  plat_per_trace: number | null;
  ducats_per_trace: number | null;
  chances: number[];
  expected_plat_shares: number[];
}

export interface OwnedRefinement {
  refinement: string;
  count: number;
}

export interface DropLocation {
  location: string;
  chance: number;
}

export interface RelicMarket {
  slug: string;
  sell: number | null;
  buy: number | null;
}

export interface Ownership {
  owned: number;
  by_refinement: OwnedRefinement[];
  all_sets_owned: boolean;
  all_items_owned: boolean;
  missing_items: number;
}

export interface PerTrace {
  value: number;
  refinement: string;
}

export interface Best {
  plat: number;
  refinement: string;
  wanted_chance: number;
  plat_per_trace: PerTrace | null;
  ducats_per_trace: PerTrace | null;
}

export interface IntactToRadiant {
  plat: number;
  ducats: number;
}

export interface RelicPlan {
  relic: string;
  unique_name: string;
  tier: string;
  image_name: string | null;
  market: RelicMarket | null;
  vaulted: boolean;
  ownership: Ownership;
  rewards: RewardBreakdown[];
  values: RefinementValue[];
  best: Best;
  intact_to_radiant: IntactToRadiant;
  favourite: boolean;
  favourite_rewards: number;
  drops: DropLocation[];
  drop_locations: number;
}

export interface MissingPart {
  unique_name: string;
  mastery: boolean;
}

export interface RelicPlannerTab {
  squad_size: number;
  only_owned: boolean;
  void_traces: number;
  missing_parts: MissingPart[];
  plans: RelicPlan[];
}

export interface RelicSource {
  relic: string;
  tier: string;
  image_name: string | null;
  vaulted: boolean;
  owned: number;
  rarity: string;
  chance: number;
}

export interface Ranked {
  store_item: string;
  unique_name: string;
  name: string;
  image_name: string | null;
  plat: number | null;
  ducats: number | null;
  set_plat: number | null;
  ownership: RewardOwnership;
  vaulted: boolean;
  favourite: boolean;
  best: boolean;
  components: RankedComponent[];
}

export interface RankedComponent {
  unique_name: string;
  name: string;
  image_name: string | null;
  owned: number;
  needed: number;
  enough: boolean;
  this_reward: boolean;
  favourite: boolean;
}

export interface RewardScreen {
  ranked: Ranked[];
  account_plat: number;
  account_ducats: number;
}
