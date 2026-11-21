import {
  ListChecks,
  type LucideIcon,
  TagPlus,
  TagX,
  TriangleAlert,
} from "lucide-react";
import { type ReactNode, useMemo } from "react";
import { Meta } from "@/components/meta";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
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

export function useSaleSafetySettings(): SaleSafetySettings {
  const settings = useSettings();
  const keep = settings.sale_safety_keep;
  const favourites = settings.sale_safety_keep_favourites;
  return useMemo(
    () => ({ sale_safety_keep: keep, sale_safety_keep_favourites: favourites }),
    [keep, favourites],
  );
}
