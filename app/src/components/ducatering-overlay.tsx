import { useEffect, useRef } from "react";
import { GameIcon } from "@/components/game-icon";
import { ItemImage } from "@/components/item-image";
import { Progress } from "@/components/ui/progress";
import { num } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { DucateringEntry, DucateringProgress } from "@/types";

const COPY: Record<
  DucateringProgress["state"]["state"],
  { title: string; description: string }
> = {
  waiting: {
    title: "Ducatering",
    description: "Waiting for the Ducat Kiosk to show its parts.",
  },
  searching: {
    title: "Ducatering",
    description: "Names are copied to your clipboard.",
  },
  finished: {
    title: "Ducatering finished",
    description: "Every item is in the sell list. Press Sell in the kiosk.",
  },
};

const EMPTY_COPY = {
  title: "Ducatering",
  description: "No parts match the Ducatering filters.",
};

function Prices({ entry, size }: { entry: DucateringEntry; size: number }) {
  return (
    <span className="flex shrink-0 flex-col items-end text-xs tabular-nums">
      <span className="text-primary flex items-center gap-0.5">
        {num(entry.plat)}
        <GameIcon name="platinum" size={size} alt="Platinum" />
      </span>
      <span className="text-accent flex items-center gap-0.5">
        {num(entry.ducats)}
        <GameIcon name="ducats" size={size} alt="Ducats" />
      </span>
    </span>
  );
}

export function DucateringOverlay({
  progress,
  compact = false,
}: {
  progress: DucateringProgress;
  compact?: boolean;
}) {
  const { state, items, sold, ducats } = progress;
  const done = state.state === "finished" ? items.length : state.position;
  const current = state.state === "searching" ? items[state.position] : null;
  const soldCount = sold.reduce((sum, sale) => sum + sale.count, 0);
  const { title, description } =
    items.length === 0 ? EMPTY_COPY : COPY[state.state];
  const list = useRef<HTMLUListElement>(null);
  useEffect(() => {
    const entry = list.current?.children[done];
    if (list.current && entry instanceof HTMLElement) {
      list.current.scrollTop = entry.offsetTop - list.current.offsetTop;
    }
  }, [done]);
  return (
    <div className="flex flex-col gap-3">
      {compact && (
        <div className="flex flex-col">
          <span className="font-semibold">{title}</span>
          <span className="text-muted-foreground text-xs">{description}</span>
        </div>
      )}
      {current && (
        <div className="bg-muted/40 flex items-center gap-4 rounded-xl border p-4">
          <ItemImage
            imageName={current.item.image_name}
            size={compact ? 48 : 64}
          />
          <span className="flex min-w-0 flex-1 flex-col gap-1">
            <span
              className={cn(
                "truncate font-semibold",
                compact ? "text-base" : "text-xl",
              )}
            >
              {current.item.name}
            </span>
            <span className="text-muted-foreground flex items-center gap-1 text-sm tabular-nums">
              Add {num(current.count)} for {num(current.count * current.ducats)}
              <GameIcon name="ducats" size={16} alt="Ducats" />
            </span>
          </span>
          <Prices entry={current} size={14} />
        </div>
      )}
      {items.length > 0 && <Progress value={(done / items.length) * 100} />}
      <ul
        ref={list}
        className={cn(
          "-mr-2 flex scroll-smooth flex-col gap-1 overflow-y-auto pr-2",
          compact ? "max-h-48" : "max-h-72",
        )}
      >
        {items.map((entry, index) => (
          <li
            key={entry.item.unique_name}
            className={cn(
              "flex items-center gap-3 rounded-md border p-2",
              current !== null && index === done && "border-primary",
              index < done && "opacity-50",
            )}
          >
            <ItemImage imageName={entry.item.image_name} size={28} />
            <span className="min-w-0 flex-1 truncate text-sm">
              {entry.item.name}
            </span>
            <span className="text-muted-foreground text-xs tabular-nums">
              {num(entry.count)}
            </span>
            <Prices entry={entry} size={12} />
          </li>
        ))}
      </ul>
      {soldCount > 0 && (
        <div className="text-muted-foreground flex items-center justify-end gap-1 text-xs">
          <span>Sold {num(soldCount)}</span> items for
          <span className="text-accent flex items-center gap-0.5 tabular-nums">
            {num(ducats)}
            <GameIcon name="ducats" size={14} alt="Ducats" />
          </span>
        </div>
      )}
    </div>
  );
}
