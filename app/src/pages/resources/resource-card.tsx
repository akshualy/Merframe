import { CheckCircle2 } from "lucide-react";
import { memo } from "react";
import { ItemImage } from "@/components/item-image";
import { Progress } from "@/components/ui/progress";
import { num } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { ResourceRow } from "@/types";

function ResourceCardInner({
  row,
  selected,
  onSelect,
}: {
  row: ResourceRow;
  selected: boolean;
  onSelect: (uniqueName: string) => void;
}) {
  const short = row.deficit > 0;
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={() => onSelect(row.unique_name)}
      className={cn(
        "bg-card flex h-full w-full cursor-pointer items-center gap-3 rounded-xl border p-3 text-left transition-colors",
        selected ? "border-primary bg-secondary/50" : "hover:border-primary/50",
      )}
    >
      <ItemImage imageName={row.image_name} size={48} alt={row.name} />
      <div className="flex min-w-0 flex-1 flex-col gap-1.5">
        <div className="flex items-center justify-between gap-2">
          <span className="line-clamp-2">{row.name}</span>
          {short ? (
            <span className="text-warning shrink-0 text-xs tabular-nums">
              Short {num(row.deficit)}
            </span>
          ) : (
            <span className="text-primary flex shrink-0 items-center gap-1 text-xs">
              <CheckCircle2 className="size-3.5" />
              Covered
            </span>
          )}
        </div>
        <Progress
          value={Math.round(
            Math.min(1, row.owned / Math.max(row.required, 1)) * 100,
          )}
          className="h-1.5"
          indicatorClassName={short ? "bg-warning" : undefined}
        />
        <div className="text-muted-foreground flex justify-between text-xs tabular-nums">
          <span>
            <span className="text-foreground">{num(row.owned)}</span> owned
          </span>
          <span>{num(row.required)} needed</span>
        </div>
      </div>
    </button>
  );
}

export const ResourceCard = memo(ResourceCardInner);
