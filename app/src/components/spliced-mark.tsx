import { Merge } from "lucide-react";
import { Hint } from "@/components/ui/hint";
import type { AttributeGrade } from "@/types";

export function SplicedMark({ attribute }: { attribute: AttributeGrade }) {
  return attribute.spliced ? (
    <Merge
      className="text-vaulted size-3.5 shrink-0 rotate-90"
      aria-label="Spliced"
    >
      <title>Spliced</title>
    </Merge>
  ) : null;
}

export function SpliceOptions({ splices }: { splices: string[] }) {
  return splices.length > 0 ? (
    <Hint as="span" className="flex items-center gap-1">
      <Merge className="size-3.5 shrink-0 rotate-90" />
      Could splice into {splices.join(", ")}
    </Hint>
  ) : null;
}
