import type { VaultStatus } from "@/types/inventory";

export interface PendingBuild {
  item_type: string;
  name: string;
  image_name: string | null;
  completes_at: string;
  remaining_secs: number;
  ready: boolean;
}

export type NodeDrop =
  | {
      kind: "relic";
      unique_name: string;
      name: string;
      image_name: string | null;
      owned: number;
      chance: number;
      vaulted: boolean;
    }
  | { kind: "purchase"; credits: number }
  | { kind: "location"; location: string; chance: number };

export interface NodeMarket {
  slug: string;
  sell: number;
}

export interface OwnedRelic {
  unique_name: string;
  name: string;
  image_name: string | null;
  owned: number;
  chance: number;
}

export interface CraftNode {
  unique_name: string;
  name: string;
  image_name: string | null;
  required: number;
  owned: number;
  per_craft: number;
  short_by: number;
  crafts_queued: number;
  craftable: boolean;
  stocked: boolean;
  covered: boolean;
  wiki_url: string | null;
  drops: NodeDrop[];
  market: NodeMarket | null;
  children: CraftNode[];
}

export interface FoundryComponent {
  unique_name: string;
  name: string;
  image_name: string | null;
  owned: number;
  required: number;
  enough: boolean;
  favourite: boolean;
  owned_relics: OwnedRelic[];
}

export interface Prime {
  vault: VaultStatus;
  resurgence: boolean;
}

export interface Progress {
  owned: boolean;
  pending: boolean;
  ready_to_build: boolean;
}

export interface MasteryGate {
  required: number | null;
  met: boolean;
}

export interface Helminth {
  ability: string;
  subsumed: boolean;
}

export interface FoundryItem {
  unique_name: string;
  name: string;
  kind: string;
  type_name: string;
  image_name: string | null;
  prime: Prime | null;
  mastered: boolean;
  progress: Progress;
  crafts_into: string[];
  mastery: MasteryGate;
  incarnon: boolean;
  helminth: Helminth | null;
  archon_shards: number;
  favourite: boolean;
  wiki_url: string | null;
  components: FoundryComponent[];
}

export interface NeededItem {
  unique_name: string;
  name: string;
  image_name: string | null;
  amount: number;
}

export interface CraftSummary {
  credits: number;
  build_secs: number;
  shortest_secs: number;
  blueprints_needed: NeededItem[];
  resources_needed: NeededItem[];
}

export interface CraftDetails {
  tree: CraftNode[];
  summary: CraftSummary;
}

export interface WorldTimer {
  name: string;
  state: string;
  ends_at: string;
  remaining_secs: number;
}

export interface FoundryTab {
  pending: PendingBuild[];
  items: FoundryItem[];
  timers: WorldTimer[];
}
