import { useEffect } from "react";
import { ItemImage, prefetchImages } from "@/components/item-image";
import { Section } from "@/components/page";
import { useNow } from "@/hooks/use-now";
import { countdown, secondsUntil } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { PendingBuild } from "@/types";

export function FoundryQueue({ builds }: { builds: PendingBuild[] }) {
  const now = useNow();

  useEffect(() => {
    prefetchImages(builds.map((build) => build.image_name));
  }, [builds]);

  return (
    <Section
      title="In the Foundry"
      description={`${builds.length} ${builds.length === 1 ? "build" : "builds"} queued`}
    >
      <div className="flex flex-wrap gap-2">
        {builds.map((build) => {
          const remaining = secondsUntil(build.completes_at, now);
          const done = remaining <= 0;
          return (
            <div
              key={build.item_type}
              className={cn(
                "bg-background flex items-center gap-2 rounded-lg border py-1.5 pr-3 pl-1.5",
                done && "border-primary/50",
              )}
            >
              <ItemImage
                imageName={build.image_name}
                size={28}
                alt={build.name}
              />
              <span className="text-sm font-medium">{build.name}</span>
              <span
                className={cn(
                  "font-mono text-xs tabular-nums",
                  done ? "text-primary" : "text-accent",
                )}
              >
                {done ? "ready" : countdown(remaining)}
              </span>
            </div>
          );
        })}
      </div>
    </Section>
  );
}
