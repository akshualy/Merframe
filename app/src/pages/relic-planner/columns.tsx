import type { ColumnDef } from "@tanstack/react-table";
import { Amount } from "@/components/currency";
import { FavouriteStar } from "@/components/favourite-star";
import { GameIcon, relicTierIcon } from "@/components/game-icon";
import { Badge } from "@/components/ui/badge";
import { Hint } from "@/components/ui/hint";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { num, percent, plat } from "@/lib/format";
import { REFINEMENTS, refinementTone } from "@/lib/relics";
import { cn } from "@/lib/utils";
import type { RefinementValue, RelicPlan, RewardBreakdown } from "@/types";

export interface RewardView {
  reward: RewardBreakdown;
  chance: number;
  squadChance: number;
  share: number;
}

export interface PlannerRow {
  plan: RelicPlan;
  value: RefinementValue;
  rewards: RewardView[];
  perTrace: number | null;
  perTraceRefinement: string | null;
  ducatsPerTrace: number | null;
  ducatTraceRefinement: string | null;
}

export function rewardViews(
  plan: RelicPlan,
  value: RefinementValue,
): RewardView[] {
  return plan.rewards.map((reward, index) => ({
    reward,
    chance: value.chances[index] ?? 0,
    squadChance: value.squad_chances[index] ?? 0,
    share: value.expected_plat_shares[index] ?? 0,
  }));
}

export function searchValue(row: PlannerRow): string {
  return `${row.plan.relic} ${row.plan.rewards.map((reward) => reward.name).join(" ")}`;
}

function HeaderHint({ label, hint }: { label: string; hint: string }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span className="underline decoration-dotted underline-offset-4">
          {label}
        </span>
      </TooltipTrigger>
      <TooltipContent>{hint}</TooltipContent>
    </Tooltip>
  );
}

export function plannerColumns(
  squadSize: number,
  showWanted: boolean,
): ColumnDef<PlannerRow>[] {
  const list: ColumnDef<PlannerRow>[] = [
    {
      id: "relic",
      header: "Relic",
      enableHiding: false,
      accessorFn: (row) => row.plan.relic,
      cell: ({ row }) => (
        <span className="flex items-center gap-2">
          <GameIcon name={relicTierIcon(row.original.plan.tier)} size={22} />
          <span className="font-medium">{row.original.plan.relic}</span>
        </span>
      ),
    },
    {
      id: "favourites",
      header: () => (
        <HeaderHint
          label="Fav"
          hint="Starred relics, and the number of their rewards you starred."
        />
      ),
      meta: { label: "Favourite" },
      accessorFn: (row) => row.plan.favourite_rewards,
      cell: ({ row }) => (
        <span className="flex items-center gap-1">
          <FavouriteStar
            uniqueName={row.original.plan.unique_name}
            favourite={row.original.plan.favourite}
            size="small"
          />
          {row.original.plan.favourite_rewards > 0 && (
            <span className="text-accent text-xs tabular-nums">
              {num(row.original.plan.favourite_rewards)}
            </span>
          )}
        </span>
      ),
    },
    {
      id: "owned",
      header: "Owned",
      meta: { numeric: true },
      accessorFn: (row) => row.plan.ownership.owned,
      cell: ({ row }) => num(row.original.plan.ownership.owned),
    },
    {
      id: "refinements",
      header: "By refinement",
      enableSorting: false,
      cell: ({ row }) => (
        <span className="flex gap-2">
          {REFINEMENTS.map((entry) => {
            const held = row.original.plan.ownership.by_refinement.find(
              (candidate) => candidate.refinement === entry,
            );
            return (
              <span
                key={entry}
                className={cn(
                  "text-xs",
                  held ? refinementTone(entry) : "text-muted-foreground/40",
                )}
              >
                {entry.slice(0, 3)} {held ? held.count : 0}
              </span>
            );
          })}
        </span>
      ),
    },
    {
      id: "expected_plat",
      header: () => (
        <HeaderHint
          label="Exp. plat"
          hint={`Platinum per run for a squad of ${squadSize}, at the refinement shown.`}
        />
      ),
      meta: { numeric: true, label: "Expected plat" },
      accessorFn: (row) => row.value.expected_plat,
      cell: ({ row }) => (
        <span className="flex flex-col justify-end">
          <Amount currency="platinum" className="font-bold">
            {plat(row.original.value.expected_plat)}
          </Amount>
          <span
            className={cn(
              "text-xs",
              refinementTone(row.original.value.refinement),
            )}
          >
            {row.original.value.refinement}
          </span>
        </span>
      ),
    },
    {
      id: "expected_ducats",
      header: () => (
        <HeaderHint
          label="Exp. ducats"
          hint="Ducats per run, at the same refinement."
        />
      ),
      meta: { numeric: true, label: "Expected ducats" },
      accessorFn: (row) => row.value.expected_ducats,
      cell: ({ row }) => num(row.original.value.expected_ducats),
    },
    {
      id: "plat_per_trace",
      header: "Plat/trace",
      meta: { numeric: true, label: "Plat per trace" },
      accessorFn: (row) => row.perTrace ?? 0,
      cell: ({ row }) => (
        <span className="flex flex-col">
          <Amount currency="platinum" plain className="justify-end">
            {row.original.perTrace === null
              ? "-"
              : row.original.perTrace.toFixed(2)}
          </Amount>
          <Hint as="span">
            {row.original.perTraceRefinement ?? "none pays"}
          </Hint>
        </span>
      ),
    },
    {
      id: "ducats_per_trace",
      header: "Ducats/trace",
      meta: { numeric: true, label: "Ducats per trace" },
      accessorFn: (row) => row.ducatsPerTrace ?? 0,
      cell: ({ row }) => (
        <span className="flex flex-col justify-end ">
          <span>
            {row.original.ducatsPerTrace === null
              ? "-"
              : row.original.ducatsPerTrace.toFixed(2)}
          </span>
          <Hint as="span">
            {row.original.ducatTraceRefinement ?? "none pays"}
          </Hint>
        </span>
      ),
    },
    {
      id: "intact_to_radiant",
      header: () => (
        <HeaderHint
          label="To radiant"
          hint="What refining from Intact to Radiant adds per run."
        />
      ),
      meta: { numeric: true, label: "Intact to radiant" },
      accessorFn: (row) => row.plan.intact_to_radiant.plat,
      cell: ({ row }) => (
        <span className="flex flex-col">
          <Amount currency="platinum" plain className="justify-end">
            {plat(row.original.plan.intact_to_radiant.plat)}
          </Amount>
          <Amount
            currency="ducats"
            plain
            className="text-muted-foreground text-xs"
          >
            {plat(row.original.plan.intact_to_radiant.ducats)}
          </Amount>
        </span>
      ),
    },
    {
      id: "missing_items",
      header: () => (
        <HeaderHint
          label="Missing"
          hint="Rewards you still need for a set and have not built or mastered. Forma never counts."
        />
      ),
      meta: { numeric: true, label: "Missing items" },
      accessorFn: (row) => row.plan.ownership.missing_items,
      cell: ({ row }) => num(row.original.plan.ownership.missing_items),
    },
  ];
  if (showWanted) {
    list.push({
      id: "wanted_chance",
      header: () => (
        <HeaderHint
          label="Wanted"
          hint={`Chance of at least one wanted part per run, for a squad of ${squadSize}.`}
        />
      ),
      meta: { numeric: true, label: "Wanted chance" },
      accessorFn: (row) => row.value.wanted_chance,
      cell: ({ row }) => (
        <span
          className={row.original.value.wanted_chance > 0 ? "text-accent" : ""}
        >
          {percent(row.original.value.wanted_chance / 100)}
        </span>
      ),
    });
  }
  list.push({
    id: "vaulted",
    header: "Vault",
    accessorFn: (row) => (row.plan.vaulted ? 1 : 0),
    cell: ({ row }) =>
      row.original.plan.vaulted ? (
        <Badge variant="vaulted">vaulted</Badge>
      ) : (
        <Badge variant="muted">available</Badge>
      ),
  });
  return list;
}
