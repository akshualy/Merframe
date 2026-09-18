import type { YesNo } from "@/lib/filters";
import { REFINEMENTS } from "@/lib/relics";
import { readStoredJson, writeStoredJson } from "@/lib/storage";
import type { RefinementValue, RelicPlan } from "@/types";

const OVERLAY_FILTERS_KEY = "merframe.relicPlanner.overlayFilters";
export const ANY_REFINEMENT = "any";
export const RADIANT = "Radiant";

export type FavouriteFilter = YesNo | "order";

export interface RelicFilters {
  squadSize: number;
  refinement: string;
  favourite: FavouriteFilter | null;
  vaulted: YesNo | null;
  setsOwned: YesNo | null;
  itemsOwned: YesNo | null;
  stacked: YesNo | null;
  refined: YesNo | null;
  tierOwned: string | null;
  wanted: string[];
}

export function partIdentity(uniqueName: string): string {
  return uniqueName.replace(/(?:Blueprint|Component)$/, "");
}

export function wantedKeysOf(wanted: string[]): Set<string> {
  return new Set(wanted.map(partIdentity));
}

export function readOverlayFilters(): RelicFilters | null {
  return readStoredJson<RelicFilters>(OVERLAY_FILTERS_KEY);
}

export function writeOverlayFilters(filters: RelicFilters) {
  writeStoredJson(OVERLAY_FILTERS_KEY, filters);
}

export function refinementValue(
  plan: RelicPlan,
  refinement: string,
): RefinementValue | undefined {
  const target =
    refinement === ANY_REFINEMENT ? plan.best.refinement : refinement;
  return plan.values.find((entry) => entry.refinement === target);
}

export function highestOwnedRefinement(plan: RelicPlan): string | undefined {
  return REFINEMENTS.filter((refinement) =>
    plan.ownership.by_refinement.some(
      (entry) => entry.refinement === refinement && entry.count > 0,
    ),
  ).at(-1);
}

export function wantedChance(
  plan: RelicPlan,
  value: RefinementValue,
  wantedKeys: Set<string>,
  squadSize: number,
): number {
  let missed = 1;
  plan.rewards.forEach((reward, index) => {
    if (!wantedKeys.has(partIdentity(reward.unique_name))) {
      return;
    }
    missed *= (1 - (value.chances[index] ?? 0) / 100) ** squadSize;
  });
  return (1 - missed) * 100;
}

function matches(flag: boolean, filter: YesNo | null): boolean {
  return filter === null || (filter === "yes") === flag;
}

function isRefined(plan: RelicPlan): boolean {
  return plan.ownership.by_refinement.some(
    (entry) => entry.refinement !== REFINEMENTS[0],
  );
}

function ownsTier(plan: RelicPlan, tier: string): boolean {
  return plan.ownership.by_refinement.some(
    (entry) => entry.refinement === tier,
  );
}

export function isFavourite(plan: RelicPlan): boolean {
  return plan.favourite || plan.favourite_rewards > 0;
}

function dropsWanted(plan: RelicPlan, wantedKeys: Set<string>): boolean {
  return plan.rewards.some((reward) =>
    wantedKeys.has(partIdentity(reward.unique_name)),
  );
}

export function matchesRelicFilters(
  plan: RelicPlan,
  filters: RelicFilters,
  wantedKeys: Set<string>,
): boolean {
  if (wantedKeys.size > 0 && !dropsWanted(plan, wantedKeys)) {
    return false;
  }
  if (filters.favourite === "yes" && !isFavourite(plan)) {
    return false;
  }
  if (filters.favourite === "no" && isFavourite(plan)) {
    return false;
  }
  if (!matches(plan.vaulted, filters.vaulted)) {
    return false;
  }
  if (!matches(plan.ownership.all_sets_owned, filters.setsOwned)) {
    return false;
  }
  if (!matches(plan.ownership.all_items_owned, filters.itemsOwned)) {
    return false;
  }
  if (!matches(plan.ownership.owned >= 10, filters.stacked)) {
    return false;
  }
  if (!matches(isRefined(plan), filters.refined)) {
    return false;
  }
  if (filters.tierOwned !== null && !ownsTier(plan, filters.tierOwned)) {
    return false;
  }
  return true;
}
