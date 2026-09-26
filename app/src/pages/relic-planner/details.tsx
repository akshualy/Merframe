import { Link } from "react-router";
import { FavouriteStar } from "@/components/favourite-star";
import { GameIcon } from "@/components/game-icon";
import { ItemImage } from "@/components/item-image";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { marketListingPath, num, percent, plat } from "@/lib/format";
import { refinementTone } from "@/lib/relics";
import { cn } from "@/lib/utils";
import type { RelicPlan } from "@/types";
import type { PlannerRow, RewardView } from "./columns";

function rarityVariant(
  rarity: string,
): "muted" | "outline" | "secondary" | "accent" {
  switch (rarity) {
    case "Rare":
      return "secondary";
    case "Uncommon":
      return "outline";
    case "Legendary":
      return "accent";
    default:
      return "muted";
  }
}

function RewardRow({ view }: { view: RewardView }) {
  const { reward, chance, squadChance, share, wanted } = view;
  return (
    <li className="flex items-center gap-2 border-t py-1.5 text-sm first:border-t-0">
      <ItemImage imageName={reward.image_name} size={24} alt={reward.name} />
      <FavouriteStar
        uniqueName={reward.unique_name}
        favourite={reward.favourite}
        size="small"
      />
      <span className="flex-1 truncate">{reward.name}</span>
      {wanted && <Badge variant="accent">wanted</Badge>}
      {!wanted && reward.ownership.needed_for_set && (
        <Badge variant="outline">needed</Badge>
      )}
      {!wanted &&
        !reward.ownership.needed_for_set &&
        reward.ownership.owned > 0 && <Badge variant="muted">owned</Badge>}
      <Badge variant={rarityVariant(reward.rarity)} className="w-20">
        {reward.rarity}
      </Badge>
      <span className="w-16 text-right text-xs">
        {percent(chance / 100, 2)}
      </span>
      <span className="w-16 text-right text-xs">
        {percent(squadChance / 100, 2)}
      </span>
      <span className="flex w-16 items-center justify-end gap-0.5 text-xs">
        {num(reward.plat)}
        <GameIcon name="platinum" size={16} alt="Platinum" />
      </span>
      <span className="text-primary flex w-20 items-center justify-end gap-0.5 text-xs font-medium">
        {plat(share)}
        <GameIcon name="platinum" size={16} alt="Platinum" />
      </span>
    </li>
  );
}

function RelicMarket({ plan }: { plan: RelicPlan }) {
  const market = plan.market;
  if (!market) {
    return null;
  }
  return (
    <div className="flex flex-wrap items-center gap-2 text-xs">
      <span className="text-muted-foreground">{plan.relic} Relic</span>
      <Button variant="outline" size="sm" asChild>
        <Link to={marketListingPath(market.slug, "sell")}>
          Sell {num(market.sell)}
          <GameIcon name="platinum" size={16} alt="Platinum" />
        </Link>
      </Button>
      <Button variant="outline" size="sm" asChild>
        <Link to={marketListingPath(market.slug, "buy")}>
          Buy {num(market.buy)}
          <GameIcon name="platinum" size={16} alt="Platinum" />
        </Link>
      </Button>
    </div>
  );
}

function DropLocations({ plan }: { plan: RelicPlan }) {
  if (plan.drops.length === 0) {
    return <Hint>This relic does not drop anywhere right now.</Hint>;
  }
  const rest = plan.drop_locations - plan.drops.length;
  return (
    <div className="flex flex-col gap-1">
      <Hint as="span">
        Drop locations
        {rest > 0
          ? `, best ${plan.drops.length} of ${plan.drop_locations}`
          : ""}
      </Hint>
      <ul className="flex flex-col">
        {plan.drops.map((drop) => (
          <li
            key={drop.location}
            className="flex items-center gap-2 border-t py-1 text-xs first:border-t-0"
          >
            <span className="flex-1 truncate">{drop.location}</span>
            <span className="w-16 text-right">
              {percent(drop.chance / 100, 2)}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

export function PlannerRowDetails({ row }: { row: PlannerRow }) {
  return (
    <div className="flex flex-col gap-2 py-2">
      <RelicMarket plan={row.plan} />
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs">
        {row.plan.values.map((entry) => (
          <span
            key={entry.refinement}
            className={cn(
              "flex items-center gap-1.5",
              entry.refinement === row.value.refinement && "font-bold",
            )}
          >
            <span className={refinementTone(entry.refinement)}>
              {entry.refinement}
            </span>
            <span className="flex items-center gap-0.5">
              {plat(entry.expected_plat)}
              <GameIcon name="platinum" size={16} alt="Platinum" />
            </span>
            <span className="text-muted-foreground">
              {num(entry.traces)} traces
              {entry.plat_per_trace === null
                ? ""
                : `, ${entry.plat_per_trace.toFixed(2)} platinum per trace`}
            </span>
          </span>
        ))}
      </div>
      <ul className="flex flex-col">
        <li className="text-muted-foreground flex items-center gap-2 pb-1 text-xs">
          <span className="w-6" />
          <span className="w-6" />
          <span className="flex flex-1 items-center gap-0.5">
            {row.value.refinement} rewards, {plat(row.value.expected_plat)}
            <GameIcon name="platinum" size={16} alt="Platinum" />
            expected in total
          </span>
          <span className="w-20">Rarity</span>
          <span className="w-16 text-right">Chance</span>
          <span className="w-16 text-right">In squad</span>
          <span className="w-16 text-right">Price</span>
          <span className="w-20 text-right">Of expected</span>
        </li>
        {row.rewards.map((view) => (
          <RewardRow key={view.reward.unique_name} view={view} />
        ))}
      </ul>
      <DropLocations plan={row.plan} />
    </div>
  );
}
