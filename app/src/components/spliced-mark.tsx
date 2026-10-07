import { Merge } from "lucide-react";
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
