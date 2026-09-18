export interface Fissure {
  node_id: string;
  node_name: string | null;
  mission_type: string;
  mission_name: string;
  tier: string;
  steel_path: boolean;
  is_storm: boolean;
  activation: string;
  expiry: string;
}

export interface ManifestItem {
  item_type: string;
  ducats: number | null;
  credits: number | null;
}

export type BaroStatus =
  | { Away: { arrives: string } }
  | {
      Present: {
        leaves: string;
        character: string;
        node_id: string;
        node_name: string | null;
        items: ManifestItem[];
      };
    };

export interface BaroOffer {
  name: string;
  ducats: number | null;
  credits: number | null;
}

export interface BaroGroup {
  name: string;
  items: BaroOffer[];
}

export interface Mission {
  mission_type: string;
  mission_name: string;
  modifier?: string;
  modifier_name?: string;
  node_id: string;
  node_name: string | null;
}

export interface Sortie {
  boss: string;
  boss_name: string;
  faction: string | null;
  activation: string;
  expiry: string;
  missions: Mission[];
}

export interface ArchonHunt {
  boss: string;
  boss_name: string;
  faction: string | null;
  activation: string;
  expiry: string;
  missions: Mission[];
}

export interface Timer {
  name: string;
  state: string;
  next_state: string;
  starts: string;
  ends: string;
}

export interface DarvoDeal {
  name: string;
  discount_percent: number;
  original_price: number;
  sale_price: number;
  amount_total: number;
  amount_sold: number;
  expiry: string;
}

export type ChallengeKind = "daily" | "weekly" | "eliteWeekly";

export interface NightwaveChallenge {
  tag: string;
  name: string;
  description: string | null;
  kind: ChallengeKind;
  activation: string;
  expiry: string;
}

export interface NightwaveSeason {
  season: number;
  phase: number;
  affiliation_tag: string;
  name: string;
  activation: string;
  expiry: string;
  challenges: NightwaveChallenge[];
}

export interface Circuit {
  rotates: string;
  normal: string[];
  hard: string[];
}

export interface ResurgenceOffering {
  name: string;
  regal_aya: number;
}

export interface PrimeResurgence {
  node_id: string;
  node_name: string | null;
  ends: string;
  offerings: ResurgenceOffering[];
}

export interface WorldStateView {
  fissures: Fissure[];
  baro: BaroStatus | null;
  baro_manifest: BaroGroup[];
  sortie: Sortie | null;
  archon_hunt: ArchonHunt | null;
  timers: Timer[];
  daily_deals: DarvoDeal[];
  circuit: Circuit | null;
  prime_resurgence: PrimeResurgence | null;
  nightwave: NightwaveSeason | null;
  fetched_at: string | null;
}
