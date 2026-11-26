import {
  ListChecks,
  type LucideIcon,
  TagPlus,
  TagX,
  TriangleAlert,
} from "lucide-react";
import { type ReactNode, useCallback, useMemo } from "react";
import { Meta } from "@/components/meta";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useAsyncData } from "@/hooks/use-async-data";
import { api } from "@/lib/bridge";
import type { FilterOption } from "@/lib/filters";
import { num } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useMarketSignedIn, useSettings } from "@/stores/app-store";
import type {
  SaleSafety,
  SaleSafetyReason,
  SaleSafetyReservation,
  SaleSafetySettings,
  SaleSafetyState,
} from "@/types";

type SaleSafetyKind = SaleSafetyState["kind"];

const STATES: Record<SaleSafetyKind, { tone: string; icon: LucideIcon }> = {
  nothing_to_list: { tone: "text-muted-foreground", icon: TagX },
  more_to_list: { tone: "text-muted-foreground", icon: TagPlus },
  all_listed: { tone: "text-muted-foreground", icon: ListChecks },
  over_listed: { tone: "text-warning", icon: TriangleAlert },
};

export type ListedSaleSafetyKind = Exclude<SaleSafetyKind, "nothing_to_list">;

export const SALE_SAFETY_FILTERS: FilterOption<ListedSaleSafetyKind>[] = [
  { value: "over_listed", label: "Too many listed" },
  { value: "more_to_list", label: "More safe to list" },
  { value: "all_listed", label: "All listed" },
];

const REASONS: Record<SaleSafetyReason, string> = {
  mastery: "you haven't mastered it",
  ownership: "you don't own one",
  favourite: "it's a favourite",
};

function saleSafetyLabel({ state, listed }: SaleSafety): string {
  switch (state.kind) {
    case "nothing_to_list":
      return "Not safe to list";
    case "more_to_list":
      return listed
        ? `${num(state.more)} more safe to list`
        : `${num(state.more)} safe to list`;
    case "all_listed":
      return "All listed";
    case "over_listed":
      return `${num(state.over)} too many listed`;
  }
}

function reservationLine({ item, via, reason }: SaleSafetyReservation) {
  return `${item}${via ? ` (needs ${via})` : ""}: ${REASONS[reason]}`;
}

export function SaleSafetyTooltip({
  safety,
  children,
}: {
  safety: SaleSafety;
  children: ReactNode;
}) {
  const { reserved, reserved_for, listed, listed_as_parts } = safety;
  const signedIn = useMarketSignedIn();
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Meta
          tabIndex={0}
          className={cn(
            STATES[safety.state.kind].tone,
            "focus-visible:ring-ring/50 rounded-sm outline-none focus-visible:ring-[3px]",
          )}
        >
          {children}
        </Meta>
      </TooltipTrigger>
      <TooltipContent className="flex flex-col gap-1">
        <span className="font-medium">{saleSafetyLabel(safety)}</span>
        {reserved > 0 ? (
          <>
            <span>{num(reserved)} needed for crafting:</span>
            <ul className="flex flex-col gap-0.5 pl-2">
              {reserved_for.map(reservationLine).map((line) => (
                <li key={line}>{line}</li>
              ))}
            </ul>
          </>
        ) : (
          <span>None needed for crafting</span>
        )}
        {listed !== null && (
          <span>
            {num(listed)} listed on warframe.market
            {listed_as_parts > 0 &&
              `, ${num(listed_as_parts)} of them as single parts`}
          </span>
        )}
        {listed === null && !signedIn && (
          <span>
            Sign in on the warframe.market page to count your sell orders
          </span>
        )}
      </TooltipContent>
    </Tooltip>
  );
}

export function SaleSafetyMark({ safety }: { safety: SaleSafety }) {
  const Icon = STATES[safety.state.kind].icon;
  return (
    <SaleSafetyTooltip safety={safety}>
      <Icon className="size-3.5" />
      {saleSafetyLabel(safety)}
    </SaleSafetyTooltip>
  );
}

export function SaleSafetyListing({
  itemId,
  quantity,
  onUse,
}: {
  itemId: string;
  quantity: string;
  onUse: (quantity: string) => void;
}) {
  const saleSafety = useSaleSafetySettings();
  const load = useCallback(
    () => api.marketSaleSafety(itemId, saleSafety),
    [itemId, saleSafety],
  );
  const { data } = useAsyncData(
    `market_sale_safety:${JSON.stringify([itemId, saleSafety])}`,
    load,
    ["inventoryUpdated", "favouriteUpdated", "marketUpdated"],
  );
  if (!data) {
    return null;
  }

  const { owned, order_limit: limit, safety } = data;
  const tooMany =
    safety.listed !== null && Number.parseInt(quantity, 10) > limit;
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-3 text-sm">
        <SaleSafetyMark safety={safety} />
        <Hint as="span">{num(owned)} owned</Hint>
      </div>
      {tooMany && (
        <div className="flex items-center gap-2">
          <Hint className="text-warning flex-1">
            {limit === 0
              ? "None of these are safe to list."
              : `That's more than the ${num(limit)} safe to list.`}
          </Hint>
          {limit > 0 && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => onUse(String(limit))}
            >
              Use {num(limit)}
            </Button>
          )}
        </div>
      )}
    </div>
  );
}

export function useSaleSafetySettings(): SaleSafetySettings {
  const settings = useSettings();
  const keep = settings.sale_safety_keep;
  const favourites = settings.sale_safety_keep_favourites;
  return useMemo(
    () => ({ sale_safety_keep: keep, sale_safety_keep_favourites: favourites }),
    [keep, favourites],
  );
}
