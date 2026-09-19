import { Star } from "lucide-react";
import { useEffect } from "react";
import { GameIcon } from "@/components/game-icon";
import { ItemImage, prefetchImages } from "@/components/item-image";
import { EmptyNote } from "@/components/page";
import { Badge } from "@/components/ui/badge";
import { num } from "@/lib/format";
import { occurrenceKeys } from "@/lib/keys";
import { cn } from "@/lib/utils";
import type { Ranked, RankedComponent, RewardScreen } from "@/types";

function Platinum({
  amount,
  size = 16,
  className,
}: {
  amount: string;
  size?: number;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "text-primary flex items-center gap-0.5 tabular-nums",
        className,
      )}
    >
      {amount}
      <GameIcon name="platinum" size={size} alt="Platinum" />
    </span>
  );
}

function Ducats({
  amount,
  size = 16,
  className,
}: {
  amount: string;
  size?: number;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "text-accent flex items-center gap-0.5 tabular-nums",
        className,
      )}
    >
      {amount}
      <GameIcon name="ducats" size={size} alt="Ducats" />
    </span>
  );
}

function RewardComponentTile({ component }: { component: RankedComponent }) {
  return (
    <span
      title={
        component.owned === null
          ? component.name
          : `${component.name}: ${num(component.owned)} of ${num(component.needed)}`
      }
      className={cn(
        "bg-background relative flex size-9 items-center justify-center rounded-full border",
        component.enough && "border-primary bg-primary/15",
        component.owned !== null && !component.enough && "opacity-60",
        component.this_reward && "ring-accent ring-2",
      )}
    >
      <ItemImage
        imageName={component.image_name}
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

function RewardTile({ reward, compact }: { reward: Ranked; compact: boolean }) {
  const keys = occurrenceKeys(
    reward.components.map((component) => component.unique_name),
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
            imageName={reward.image_name}
            size={32}
            alt={reward.name}
          />
        )}
        <span className="flex min-w-0 flex-1 items-start gap-1.5">
          <span className="line-clamp-2 text-sm leading-tight font-medium">
            {reward.name}
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
        <Platinum amount={num(reward.plat)} className="text-sm font-bold" />
        <Ducats amount={num(reward.ducats)} className="text-sm" />
        {reward.vaulted && (
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
          <Platinum
            amount={num(reward.set_plat)}
            size={14}
            className="font-medium"
          />
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
      ...screen.ranked.map((reward) => reward.image_name),
      ...screen.ranked.flatMap((reward) =>
        reward.components.map((component) => component.image_name),
      ),
    ]);
  }, [screen]);

  if (screen.ranked.length === 0) {
    return <EmptyNote>The reward prices could not be loaded.</EmptyNote>;
  }

  const keys = occurrenceKeys(screen.ranked.map((reward) => reward.store_item));
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
            <Platinum
              amount={num(screen.account.plat)}
              size={14}
              className="font-medium"
            />
          </span>
          <span className="flex items-center gap-1">
            Ducats
            <Ducats amount={num(screen.account.ducats)} size={14} />
          </span>
        </div>
      )}
    </div>
  );
}
