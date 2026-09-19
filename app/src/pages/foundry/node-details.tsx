import { Archive, ExternalLink } from "lucide-react";
import { useCallback } from "react";
import { useNavigate } from "react-router";
import { GameIcon } from "@/components/game-icon";
import { ItemImage } from "@/components/item-image";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { api, reportError } from "@/lib/bridge";
import { marketUrl, num, percent } from "@/lib/format";
import { occurrenceKeys } from "@/lib/keys";
import { cn } from "@/lib/utils";
import type { CraftNode, NodeDrop, NodeMarket } from "@/types";

function dropKey(drop: NodeDrop): string {
  if (drop.kind === "relic") {
    return drop.unique_name;
  }
  return drop.kind === "purchase" ? "purchase" : drop.location;
}

export function WikiButton({ url }: { url: string }) {
  const handleOpen = useCallback(async () => {
    try {
      await api.openUrl(url);
    } catch (error) {
      reportError(error);
    }
  }, [url]);
  return (
    <Button variant="ghost" size="sm" onClick={handleOpen}>
      Wiki
      <ExternalLink className="size-3.5" />
    </Button>
  );
}

function DropRow({
  drop,
  onOpenRelic,
}: {
  drop: NodeDrop;
  onOpenRelic: (name: string) => void;
}) {
  if (drop.kind === "relic") {
    return (
      <button
        type="button"
        onClick={() => onOpenRelic(drop.name)}
        className="hover:bg-secondary/50 flex w-full cursor-pointer items-center gap-2 py-1 text-left text-xs"
      >
        <ItemImage imageName={drop.image_name} size={24} alt={drop.name} />
        {drop.vaulted && <Archive className="text-vaulted size-3.5 shrink-0" />}
        <span className={cn("flex-1 truncate", drop.vaulted && "text-vaulted")}>
          {drop.name}
        </span>
        {drop.owned > 0 && (
          <Badge variant="secondary">x{num(drop.owned)}</Badge>
        )}
        <span className="w-16 text-right">{percent(drop.chance / 100, 1)}</span>
      </button>
    );
  }
  if (drop.kind === "purchase") {
    return (
      <span className="flex items-center gap-2 py-1 text-xs">
        <span className="flex-1">Bought with Credits</span>
        <span className="tabular-nums">{num(drop.credits)} Credits</span>
      </span>
    );
  }
  return (
    <span className="flex items-center gap-2 py-1 text-xs">
      <span className="flex-1 truncate">{drop.location}</span>
      <span className="w-16 text-right">{percent(drop.chance / 100, 2)}</span>
    </span>
  );
}

function DropLocations({ drops }: { drops: NodeDrop[] }) {
  const navigate = useNavigate();
  const openRelic = useCallback(
    (name: string) => navigate(`/relics?search=${encodeURIComponent(name)}`),
    [navigate],
  );
  const keys = occurrenceKeys(drops.map(dropKey));
  if (drops.length === 0) {
    return <Hint>No known drop locations</Hint>;
  }
  return (
    <div className="flex flex-col gap-1">
      <Hint as="span">Drop locations</Hint>
      <ul className="flex max-h-64 flex-col overflow-y-auto">
        {drops.map((drop, position) => (
          <li key={keys[position]} className="border-t first:border-t-0">
            <DropRow drop={drop} onOpenRelic={openRelic} />
          </li>
        ))}
      </ul>
    </div>
  );
}

function MarketButton({ market }: { market: NodeMarket }) {
  const handleOpen = useCallback(async () => {
    try {
      await api.openUrl(marketUrl(market.slug));
    } catch (error) {
      reportError(error);
    }
  }, [market.slug]);
  return (
    <Button variant="outline" size="sm" onClick={handleOpen}>
      Buy {num(market.sell)}
      <GameIcon name="platinum" size={16} alt="Platinum" />
    </Button>
  );
}

function Counts({ node }: { node: CraftNode }) {
  const counts: [number, string, string][] = [
    [node.owned, "owned", "text-primary"],
    [node.crafts_queued, "can be crafted", "text-accent"],
    [node.short_by, "missing", "text-warning"],
  ];
  return (
    <span className="flex flex-wrap items-center gap-x-3 text-xs tabular-nums">
      {counts
        .filter(([count]) => count > 0)
        .map(([count, label, tone]) => (
          <span key={label} className={tone}>
            {num(count)} {label}
          </span>
        ))}
    </span>
  );
}

export function NodeDetails({ node }: { node: CraftNode }) {
  return (
    <div className="flex flex-col gap-2 border-t pt-3">
      <div className="flex flex-wrap items-center gap-2">
        <ItemImage imageName={node.image_name} size={22} alt={node.name} />
        <span className="text-sm font-medium">{node.name}</span>
        <Hint as="span" className="tabular-nums">
          x{num(node.required)}
        </Hint>
        <Counts node={node} />
        <span className="ml-auto flex items-center gap-1">
          {node.market && <MarketButton market={node.market} />}
          {node.wiki_url && <WikiButton url={node.wiki_url} />}
        </span>
      </div>
      <DropLocations drops={node.drops} />
    </div>
  );
}
