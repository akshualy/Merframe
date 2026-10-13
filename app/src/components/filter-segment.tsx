import { Lock } from "lucide-react";
import { FilterField } from "@/components/filter-grid";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { YesNo } from "@/lib/filters";
import type { Facet } from "@/lib/foundry-filters";

const ANY_VALUE = "any";

function LockNote({ reason }: { reason: string }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Lock className="size-3" />
      </TooltipTrigger>
      <TooltipContent>{reason}</TooltipContent>
    </Tooltip>
  );
}

export function FilterSegment({
  label,
  yes,
  no,
  facet,
  value,
  fixed,
  onChange,
}: {
  label: string;
  yes: string;
  no: string;
  facet: Facet;
  value: YesNo | null;
  fixed: boolean;
  onChange: (value: YesNo | null) => void;
}) {
  const reason =
    value === null
      ? `Nothing in this view answers ${label}`
      : `${label} is ${value === "yes" ? yes : no} for everything in this view`;
  return (
    <FilterField label={label} note={fixed && <LockNote reason={reason} />}>
      <ToggleGroup
        type="single"
        aria-label={label}
        value={value ?? ANY_VALUE}
        disabled={fixed}
        onValueChange={(next) => {
          if (next === "") {
            return;
          }

          onChange(next === ANY_VALUE ? null : (next as YesNo));
        }}
      >
        <ToggleGroupItem
          value="yes"
          aria-label={yes}
          disabled={facet.yes === 0}
        >
          {yes}
        </ToggleGroupItem>
        <ToggleGroupItem
          value={ANY_VALUE}
          aria-label="Any"
          className="data-[state=on]:bg-transparent data-[state=on]:text-foreground data-[state=on]:shadow-none"
        >
          Any
        </ToggleGroupItem>
        <ToggleGroupItem value="no" aria-label={no} disabled={facet.no === 0}>
          {no}
        </ToggleGroupItem>
      </ToggleGroup>
    </FilterField>
  );
}
