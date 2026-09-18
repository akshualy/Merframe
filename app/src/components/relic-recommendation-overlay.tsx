import { Star } from "lucide-react";
import { GameIcon, relicTierIcon } from "@/components/game-icon";
import { EmptyNote } from "@/components/page";
import { Badge } from "@/components/ui/badge";
import { Hint } from "@/components/ui/hint";
import { percent, plat } from "@/lib/format";
import {
  highestOwnedRefinement,
  isFavourite,
  matchesRelicFilters,
  RADIANT,
  readOverlayFilters,
  refinementValue,
  wantedChance,
  wantedKeysOf,
} from "@/lib/relic-filters";
import { refinementTone } from "@/lib/relics";
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

export function overlaySquadSize(): number {
  return readOverlayFilters()?.squadSize ?? 4;
}

function recommended(
  tier: string | null,
  plans: RelicPlan[],
  refinement: RecommendationRefinement,
  limit: number,
) {
  const filters = readOverlayFilters();
  const wantedKeys = wantedKeysOf(filters?.wanted ?? []);
  const squadSize = overlaySquadSize();
  const rows: RecommendedRelic[] = [];
  for (const plan of plans) {
    if (tier && plan.tier !== tier) {
      continue;
    }
    if (filters && !matchesRelicFilters(plan, filters, wantedKeys)) {
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
      chanceOfWanted:
        wantedKeys.size === 0
          ? null
          : wantedChance(plan, value, wantedKeys, squadSize),
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
  compact = false,
}: {
  tier: string | null;
  plans: RelicPlan[];
  refinement: RecommendationRefinement;
  limit: number;
  compact?: boolean;
}) {
  const recommendedRows = recommended(tier, plans, refinement, limit);
  const squadSize = overlaySquadSize();
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
        {squadSize} {squadSize === 1 ? "player" : "players"} with this relic
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
              {isFavourite(row.plan) && (
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
            <span className="text-primary flex w-16 shrink-0 items-center justify-end gap-0.5 text-sm font-bold tabular-nums">
              {plat(row.plat)}
              <GameIcon name="platinum" size={16} alt="Platinum" />
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
