import { Star } from "lucide-react";
import { useMemo, useSyncExternalStore } from "react";
import { Amount } from "@/components/currency";
import { GameIcon, relicTierIcon } from "@/components/game-icon";
import { EmptyNote } from "@/components/page";
import { Badge } from "@/components/ui/badge";
import { Hint } from "@/components/ui/hint";
import { counted, percent, plat } from "@/lib/format";
import {
  highestOwnedRefinement,
  matchesRelicFilters,
  RADIANT,
  type RelicFilters,
  refinementValue,
  showsWanted,
  storedOverlayFilters,
  subscribeOverlayFilters,
} from "@/lib/relic-filters";
import { refinementTone } from "@/lib/relics";
import { parseStoredJson } from "@/lib/storage";
import { cn } from "@/lib/utils";
import type { RecommendationRefinement, RelicPlan } from "@/types";

interface RecommendedRelic {
  plan: RelicPlan;
  refinement: string;
  plat: number;
  chanceOfWanted: number | null;
}

function valuedAt(plan: RelicPlan, refinement: RecommendationRefinement) {
  if (refinement === "radiant") {
    return refinementValue(plan, RADIANT);
  }
  const owned = highestOwnedRefinement(plan);
  return owned ? refinementValue(plan, owned) : undefined;
}

export function useOverlayFilters(): RelicFilters | null {
  const raw = useSyncExternalStore(
    subscribeOverlayFilters,
    storedOverlayFilters,
  );
  return useMemo(
    () => (raw === null ? null : parseStoredJson<RelicFilters>(raw)),
    [raw],
  );
}

function recommended(
  plans: RelicPlan[],
  refinement: RecommendationRefinement,
  limit: number,
  filters: RelicFilters | null,
) {
  const showWanted = showsWanted(plans);
  const rows: RecommendedRelic[] = [];
  for (const plan of plans) {
    if (filters && !matchesRelicFilters(plan, filters)) {
      continue;
    }
    const value = valuedAt(plan, refinement);
    if (!value) {
      continue;
    }
    rows.push({
      plan,
      refinement: value.refinement,
      plat: value.expected_plat,
      chanceOfWanted: showWanted ? value.wanted_chance : null,
    });
  }
  const orderFavouritesFirst = filters?.favourite === "order";
  rows.sort(
    (left, right) =>
      (orderFavouritesFirst
        ? right.plan.favourite_rewards - left.plan.favourite_rewards
        : 0) ||
      (right.chanceOfWanted ?? 0) - (left.chanceOfWanted ?? 0) ||
      right.plat - left.plat ||
      left.plan.relic.localeCompare(right.plan.relic),
  );
  return rows.slice(0, limit);
}

export function RelicRecommendationOverlay({
  tier,
  plans,
  refinement,
  limit,
  filters,
  squadSize,
  compact = false,
}: {
  tier: string | null;
  plans: RelicPlan[];
  refinement: RecommendationRefinement;
  limit: number;
  filters: RelicFilters | null;
  squadSize: number;
  compact?: boolean;
}) {
  const recommendedRows = recommended(plans, refinement, limit, filters);
  const valuedWord = refinement === "owned" ? "Owned refinement" : RADIANT;

  if (recommendedRows.length === 0) {
    return (
      <EmptyNote>
        {tier
          ? `You own no ${tier} relic that matches your filters.`
          : "No owned relic matches your filters."}
      </EmptyNote>
    );
  }

  return (
    <div className={cn("flex flex-col", compact ? "gap-1" : "gap-2")}>
      <Hint>
        Values at{" "}
        <span className={refinementTone(valuedWord)}>{valuedWord}</span>,{" "}
        {counted(squadSize, "player")} with this relic
      </Hint>
      <ul className={cn("flex flex-col", compact ? "gap-1" : "gap-2")}>
        {recommendedRows.map((row) => (
          <li
            key={row.plan.relic}
            className={cn(
              "flex items-center rounded-lg border",
              compact ? "gap-2 px-2 py-0.5" : "gap-3 px-3 py-2",
            )}
          >
            <GameIcon
              name={relicTierIcon(row.plan.tier)}
              size={compact ? 16 : 22}
              alt={row.plan.tier}
            />
            <span className="text-muted-foreground w-7 shrink-0 text-right text-xs tabular-nums">
              x{row.plan.ownership.owned}
            </span>
            <span className="flex min-w-0 flex-1 items-center gap-1.5">
              <span className="truncate text-sm font-medium">
                {row.plan.relic}
              </span>
              {row.plan.any_favourite && (
                <Star
                  className="text-accent size-3.5 shrink-0"
                  fill="currentColor"
                  aria-label="Favourite"
                />
              )}
              {row.plan.vaulted && !compact && (
                <Badge variant="vaulted">vaulted</Badge>
              )}
            </span>
            {!compact && (
              <span
                className={cn(
                  "w-24 shrink-0 text-right text-xs",
                  refinementTone(row.refinement),
                )}
              >
                {row.refinement}
              </span>
            )}
            {row.chanceOfWanted !== null && (
              <span className="text-accent w-14 shrink-0 text-right text-xs tabular-nums">
                {percent(row.chanceOfWanted / 100)}
              </span>
            )}
            <Amount
              currency="platinum"
              className="w-16 shrink-0 justify-end text-sm font-bold"
            >
              {plat(row.plat)}
            </Amount>
          </li>
        ))}
      </ul>
    </div>
  );
}
