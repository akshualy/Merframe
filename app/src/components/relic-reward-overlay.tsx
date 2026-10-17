import { Star } from "lucide-react";
import { useEffect } from "react";
import { Amount } from "@/components/currency";
import { ItemImage, prefetchImages } from "@/components/item-image";
import { Badge } from "@/components/ui/badge";
import { num } from "@/lib/format";
import { occurrenceKeys } from "@/lib/keys";
import { cn } from "@/lib/utils";
import type { RankedComponent, RankedReward, RewardScreen } from "@/types";

function RewardComponentTile({ component }: { component: RankedComponent }) {
  return (
    <span
      title={
        component.owned === null
          ? component.item.name
          : `${component.item.name}: ${num(component.owned)} of ${num(component.needed)}`
      }
      className={cn(
        "bg-background relative flex size-9 items-center justify-center rounded-full border",
        component.enough && "border-primary bg-primary/15",
        component.owned !== null && !component.enough && "opacity-60",
        component.this_reward && "ring-accent ring-2",
      )}
    >
      <ItemImage
        imageName={component.item.image_name}
        size={24}
        className="bg-transparent"
      />
      {component.favourite && (
        <Star
          className="text-accent absolute -top-1 -right-1 size-3"
          fill="currentColor"
          aria-label="Favourite"
        />
      )}
      {component.owned !== null && (
        <span className="bg-secondary text-secondary-foreground absolute -right-1 -bottom-1 min-w-4 rounded-full px-1 text-center text-xs leading-4 font-medium tabular-nums">
          {num(component.owned)}
        </span>
      )}
    </span>
  );
}

function RewardTile({
  reward,
  compact,
}: {
  reward: RankedReward;
  compact: boolean;
}) {
  const keys = occurrenceKeys(
    reward.components.map((component) => component.item.unique_name),
  );
  return (
    <li
      className={cn(
        "flex w-56 flex-col gap-1.5 rounded-lg border",
        compact ? "px-2 py-1.5" : "px-3 py-2",
        reward.best && "border-primary",
      )}
    >
      <span className="flex items-center gap-2">
        {!compact && (
          <ItemImage
            imageName={reward.item.image_name}
            size={32}
            alt={reward.item.name}
          />
        )}
        <span className="flex min-w-0 flex-1 items-start gap-1.5">
          <span className="line-clamp-2 text-sm leading-tight font-medium">
            {reward.item.name}
          </span>
          {reward.favourite && (
            <Star
              className="text-accent mt-0.5 size-3.5 shrink-0"
              fill="currentColor"
              aria-label="Favourite"
            />
          )}
        </span>
      </span>

      <span className="flex items-center gap-2">
        <Amount currency="platinum" className="text-sm font-bold">
          {num(reward.plat)}
        </Amount>
        <Amount currency="ducats" className="text-sm">
          {num(reward.item.ducats)}
        </Amount>
        {reward.item.vault === "vaulted" && (
          <Badge variant="vaulted" className="ml-auto">
            Vaulted
          </Badge>
        )}
      </span>

      {reward.ownership && (
        <span className="flex items-center justify-between gap-2 text-xs">
          <span className={reward.ownership.parent_owned ? "" : "text-warning"}>
            {reward.ownership.parent_owned ? "Crafted" : "Not crafted"}
          </span>
          <span className="text-muted-foreground tabular-nums">
            {num(reward.ownership.owned)}/{num(reward.ownership.needed)}
          </span>
        </span>
      )}

      {reward.components.length > 0 && (
        <span className="flex flex-wrap items-center gap-1.5 pt-0.5">
          {reward.components.map((component, index) => (
            <RewardComponentTile key={keys[index]} component={component} />
          ))}
        </span>
      )}

      {reward.set_plat !== null && (
        <span className="flex items-center gap-1.5 text-xs">
          <span className="text-muted-foreground">Set</span>
          <Amount currency="platinum" size={14} className="font-medium">
            {num(reward.set_plat)}
          </Amount>
        </span>
      )}
    </li>
  );
}

export function RelicRewardOverlay({
  screen,
  compact = false,
}: {
  screen: RewardScreen;
  compact?: boolean;
}) {
  useEffect(() => {
    prefetchImages([
      ...screen.ranked.map((reward) => reward.item.image_name),
      ...screen.ranked.flatMap((reward) =>
        reward.components.map((component) => component.item.image_name),
      ),
    ]);
  }, [screen]);

  const keys = occurrenceKeys(
    screen.ranked.map((reward) => reward.item.unique_name),
  );
  return (
    <div className="flex flex-col gap-2">
      <ul className="flex flex-wrap justify-center gap-2">
        {screen.ranked.map((reward, index) => (
          <RewardTile key={keys[index]} reward={reward} compact={compact} />
        ))}
      </ul>
      {screen.account && (
        <div className="text-muted-foreground flex items-center justify-end gap-4 text-xs">
          <span className="flex items-center gap-1">
            Platinum
            <Amount currency="platinum" size={14} className="font-medium">
              {num(screen.account.plat)}
            </Amount>
          </span>
          <span className="flex items-center gap-1">
            Ducats
            <Amount currency="ducats" size={14}>
              {num(screen.account.ducats)}
            </Amount>
          </span>
        </div>
      )}
    </div>
  );
}
