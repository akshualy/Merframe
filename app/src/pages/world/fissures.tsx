import { RelicTierIcon } from "@/components/game-icon";
import { EmptyNote } from "@/components/page";
import { Badge } from "@/components/ui/badge";
import { Hint } from "@/components/ui/hint";
import { secondsUntil } from "@/lib/format";
import { isRelicTier, RELIC_TIERS } from "@/lib/relics";
import { cn } from "@/lib/utils";
import { splitNode, timeLeft } from "@/lib/world";
import type { Fissure } from "@/types";

function FissureRow({ fissure, now }: { fissure: Fissure; now: number }) {
  const remaining = secondsUntil(fissure.expiry, now);
  const { node, planet } = splitNode(fissure.node_name, fissure.node_id);
  return (
    <li className="flex items-center gap-3 px-3 py-2">
      <span className="min-w-0 flex-1 truncate text-sm">
        <span className="font-semibold">{fissure.mission_name}</span>
        <span className="text-muted-foreground">
          {" "}
          ({node}
          {planet ? `, ${planet}` : ""})
        </span>
      </span>
      {fissure.steel_path && <Badge variant="secondary">Steel Path</Badge>}
      {fissure.is_storm && <Badge variant="secondary">Void storm</Badge>}
      <span
        className={cn(
          "w-20 shrink-0 text-right font-mono text-sm tabular-nums",
          remaining < 5 * 60 ? "text-warning" : "text-accent",
        )}
      >
        {timeLeft(remaining)}
      </span>
    </li>
  );
}

export function FissuresByTier({
  fissures,
  now,
}: {
  fissures: Fissure[];
  now: number;
}) {
  const tiers: string[] = [...RELIC_TIERS];
  for (const fissure of fissures) {
    if (!tiers.includes(fissure.tier)) {
      tiers.push(fissure.tier);
    }
  }
  const grouped = tiers
    .map(
      (tier) =>
        [tier, fissures.filter((fissure) => fissure.tier === tier)] as const,
    )
    .filter(([, list]) => list.length > 0);

  if (grouped.length === 0) {
    return <EmptyNote>No fissures match this filter right now.</EmptyNote>;
  }
  return (
    <div className="flex flex-col gap-3">
      {grouped.map(([tier, list]) => (
        <div
          key={tier}
          className="bg-background overflow-hidden rounded-xl border"
        >
          <div className="flex items-center gap-2 border-b px-3 py-2">
            <RelicTierIcon tier={tier} size={22} />
            {isRelicTier(tier) && <Badge variant="secondary">{tier}</Badge>}
            <Hint as="span">
              {list.length} {list.length === 1 ? "fissure" : "fissures"}
            </Hint>
          </div>
          <ul className="divide-y">
            {list.map((fissure) => (
              <FissureRow
                key={`${fissure.node_id}-${fissure.expiry}-${fissure.steel_path}`}
                fissure={fissure}
                now={now}
              />
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}
