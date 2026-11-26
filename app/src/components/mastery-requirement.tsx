import { Lock } from "lucide-react";
import { Meta } from "@/components/meta";
import { cn } from "@/lib/utils";
import type { MasteryGate } from "@/types";

export function MasteryRequirement({
  gate,
  unmetOnly = false,
}: {
  gate: MasteryGate;
  unmetOnly?: boolean;
}) {
  if (gate.required === null || gate.required <= 1 || (unmetOnly && gate.met)) {
    return null;
  }

  return (
    <Meta
      title={
        gate.met
          ? `Requires Mastery Rank ${gate.required}`
          : `Requires Mastery Rank ${gate.required}, yours is too low`
      }
      className={cn(
        "shrink-0 text-xs font-normal",
        gate.met ? "text-muted-foreground" : "text-warning",
      )}
    >
      {!gate.met && <Lock className="size-3.5" />}
      MR {gate.required}
    </Meta>
  );
}
