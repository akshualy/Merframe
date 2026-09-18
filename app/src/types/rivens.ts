export interface AttributeGrade {
  tag: string;
  name: string | null;
  slug: string | null;
  unit: string | null;
  prefix: string | null;
  suffix: string | null;
  localization: string | null;
  value: number;
  percentile: number;
  grade: string;
  multiplier: number;
  rolled: number | null;
  display: number | null;
  min: number | null;
  max: number | null;
  curse: boolean;
}

export interface StatMatch {
  abbr: string;
  name: string | null;
  matches: boolean;
}

export interface AlternativeMatch {
  mandatory: StatMatch[];
  optional: StatMatch[];
  optional_needed: number;
  complete: boolean;
}

export interface GoodRollView {
  alternatives: AlternativeMatch[];
  accepted_bad: StatMatch[];
  matches: boolean;
}

export interface PendingRoll {
  name: string | null;
  rerolls: number;
  polarity: string | null;
  grade: number;
  attributes: AttributeGrade[];
  good_roll: GoodRollView | null;
}

export interface RivenRow {
  item_id: string;
  item_type: string;
  riven_type: string | null;
  weapon_class: string | null;
  name: string | null;
  weapon: string | null;
  weapon_path: string | null;
  weapon_slug: string | null;
  image_name: string | null;
  disposition: number | null;
  disposition_weapon: string | null;
  unveiled: boolean;
  rank: number;
  rank_required: number | null;
  rerolls: number;
  polarity: string | null;
  grade: number;
  attributes: AttributeGrade[];
  good_roll: GoodRollView | null;
  listed_in_wfm: boolean;
  pending: PendingRoll | null;
}

export interface ListingChoices {
  direct: boolean;
  sellingPrice: number;
  startingPrice: number;
  buyoutPrice: number;
  minReputation: number;
  note: string | null;
  private: boolean;
  maxRankStats: boolean;
}

export interface VeiledRiven {
  riven_id: string;
  item_type: string;
  name: string | null;
  weapon_class: string | null;
  image_name: string | null;
  count: number;
  progress: number;
  required: number;
  pre_veiled: boolean;
}

export interface VeiledGroup {
  challenge_id: string;
  challenge: string;
  complication: string | null;
  count: number;
  rivens: VeiledRiven[];
}

export interface ComparedStat {
  slug: string;
  positive: boolean;
}

export interface ComparableAttribute {
  name: string;
  abbr: string | null;
  unit: string | null;
  value: number;
  positive: boolean;
  shared: boolean;
}

export interface ComparableListing {
  id: string;
  price: number;
  direct_sell: boolean;
  name: string;
  rerolls: number;
  rank: number;
  mastery: number;
  polarity: string;
  similarity: number;
  attributes: ComparableAttribute[];
}

export interface RivenComparables {
  weapon_slug: string;
  updated_at: number;
  truncated: boolean;
  listed: number;
  lowest: number | null;
  listings: ComparableListing[];
}

export interface RivensTab {
  veiled: VeiledGroup[];
  unveiled: RivenRow[];
  attribution: string | null;
}
