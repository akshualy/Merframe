import { Anvil, Check } from "lucide-react";
import type { ReactNode } from "react";
import { GameIcon } from "@/components/game-icon";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { WorldOfferOwnership } from "@/types";

const MARKS: Record<
  Exclude<WorldOfferOwnership, "missing">,
  { label: string; icon: ReactNode }
> = {
  owned: {
    label: "Owned",
    icon: <Check className="text-accent size-3.5" />,
  },
  mastered: {
    label: "Mastered",
    icon: <GameIcon name="mastered" size={20} />,
  },
  in_foundry: {
    label: "In Foundry",
    icon: <Anvil className="text-accent size-3.5" />,
  },
};

export function OwnershipMark({
  ownership,
}: {
  ownership: WorldOfferOwnership | null;
}) {
  if (ownership === null || ownership === "missing") {
    return null;
  }

  const { label, icon } = MARKS[ownership];
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span role="img" aria-label={label} className="flex shrink-0">
          {icon}
        </span>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}
