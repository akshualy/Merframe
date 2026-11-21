import {
  ListChecks,
  type LucideIcon,
  TagPlus,
  TagX,
  TriangleAlert,
} from "lucide-react";
import { type ReactNode, useMemo } from "react";
import { Meta } from "@/components/meta";
import { SelectField } from "@/components/option-select";
import { SwitchField } from "@/components/ui/switch";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { FilterOption } from "@/lib/filters";
import { num } from "@/lib/format";
import { cn } from "@/lib/utils";
import {
  useAppStore,
  useMarketSignedIn,
  useSettings,
} from "@/stores/app-store";
import type {
  SaleSafety,
  SaleSafetyKeep,
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

const KEEP_OPTIONS: FilterOption<SaleSafetyKeep>[] = [
  { value: "unmastered", label: "Unbuilt, unmastered items" },
  { value: "unbuilt", label: "All unbuilt items" },
  { value: "nothing", label: "Nothing" },
];

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

export function SaleSafetyKeepFields() {
  const settings = useSettings();
  const saveSettings = useAppStore((state) => state.saveSettings);
  return (
    <div className="flex flex-wrap items-center gap-x-6 gap-y-3">
      <SelectField
        id="sale-safety-keep"
        label="Keep parts for"
        inline
        value={settings.sale_safety_keep}
        options={KEEP_OPTIONS}
        onChange={(value) => saveSettings({ sale_safety_keep: value })}
        size="sm"
        className="w-72"
      />
      <SwitchField
        id="sale-safety-keep-favourites"
        label="Keep parts for favourites"
        checked={settings.sale_safety_keep_favourites}
        onChange={(checked) =>
          saveSettings({ sale_safety_keep_favourites: checked })
        }
      />
    </div>
  );
}
