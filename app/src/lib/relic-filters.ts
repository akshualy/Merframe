import type { YesNo } from "@/lib/filters";
import { REFINEMENTS } from "@/lib/relics";
import { readStored, writeStoredJson } from "@/lib/storage";
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

export function storedOverlayFilters(): string | null {
  return readStored(OVERLAY_FILTERS_KEY);
}

export function writeOverlayFilters(filters: RelicFilters) {
  writeStoredJson(OVERLAY_FILTERS_KEY, filters);
}

export function subscribeOverlayFilters(onChange: () => void) {
  const listener = (event: StorageEvent) => {
    if (event.key === OVERLAY_FILTERS_KEY) {
      onChange();
    }
  };
  window.addEventListener("storage", listener);
  return () => window.removeEventListener("storage", listener);
}

export function refinementValue(
  plan: RelicPlan,
  refinement: string,
): RefinementValue | undefined {
  const target =
    refinement === ANY_REFINEMENT ? plan.best.refinement : refinement;
  return plan.values.find((entry) => entry.refinement === target);
}

export function showsWanted(plans: RelicPlan[]): boolean {
  return plans.some((plan) => plan.rewards.some((reward) => reward.wanted));
}

export function highestOwnedRefinement(plan: RelicPlan): string | undefined {
  return REFINEMENTS.filter((refinement) =>
    plan.ownership.by_refinement.some(
      (entry) => entry.refinement === refinement && entry.count > 0,
    ),
  ).at(-1);
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

export function matchesRelicFilters(
  plan: RelicPlan,
  filters: RelicFilters,
): boolean {
  if (filters.favourite === "yes" && !plan.any_favourite) {
    return false;
  }
  if (filters.favourite === "no" && plan.any_favourite) {
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
