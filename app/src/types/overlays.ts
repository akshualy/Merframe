import type { RivenRow } from "@/types/rivens";
import type { RecommendationRefinement } from "@/types/settings";

export interface RewardTrigger {
  relic: string | null;
  rewards: string[];
}

export interface RecommendationTrigger {
  tier: string | null;
  refinement: RecommendationRefinement;
  count: number;
}

export interface RivenTrigger {
  item_type: string;
  before: RivenRow | null;
  linked: RivenRow | null;
}

export interface OverlayState {
  seq: number;
  opacity: number;
  reward: RewardTrigger | null;
  recommendation: RecommendationTrigger | null;
  riven: RivenTrigger | null;
}
