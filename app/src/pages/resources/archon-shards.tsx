import { ItemImage } from "@/components/item-image";
import { CardGrid, Section } from "@/components/page";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { num, numBig } from "@/lib/format";
import type { ShardRow } from "@/types";

function ShardHolders({ shard }: { shard: ShardRow }) {
  const label = `Equipped in ${num(shard.holders.length)} ${shard.holders.length === 1 ? "Warframe" : "Warframes"}`;
  return (
    <Dialog>
      <DialogTrigger className="text-muted-foreground hover:text-foreground cursor-pointer text-left text-xs transition-colors">
        {label}
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{shard.name}</DialogTitle>
          <DialogDescription>{label}</DialogDescription>
        </DialogHeader>
        <ul className="-mr-2 flex max-h-96 flex-col gap-1 overflow-y-auto pr-2">
          {shard.holders.map((holder) => (
            <li
              key={holder.item_id}
              className="flex items-center gap-3 rounded-md border p-2 text-sm"
            >
              <ItemImage imageName={holder.image_name} size={40} />
              <span className="min-w-0 flex-1 truncate">{holder.name}</span>
              <span className="text-muted-foreground text-xs tabular-nums">
                {[
                  holder.normal > 0 && `${num(holder.normal)} normal`,
                  holder.tauforged > 0 && `${num(holder.tauforged)} Tauforged`,
                ]
                  .filter(Boolean)
                  .join(", ")}
              </span>
            </li>
          ))}
        </ul>
      </DialogContent>
    </Dialog>
  );
}

const sum = (counts: number[]) => counts.reduce((a, b) => a + b, 0);

function Counts({
  label,
  free,
  equipped,
}: {
  label: string;
  free: number;
  equipped: number;
}) {
  return (
    <div className="flex justify-between gap-3">
      <span>{label}</span>
      <span>
        <span className="text-foreground">{num(free)}</span> free,{" "}
        <span className="text-foreground">{num(equipped)}</span> equipped
      </span>
    </div>
  );
}

export function ArchonShards({ shards }: { shards: ShardRow[] }) {
  return (
    <Section title="Archon Shards">
      <CardGrid>
        {shards.map((shard) => (
          <div
            key={shard.unique_name}
            className="bg-card flex items-start gap-3 rounded-xl border p-3"
          >
            <ItemImage
              imageName={shard.image_name}
              size={48}
              alt={shard.name}
            />
            <div className="flex min-w-0 flex-1 flex-col gap-1.5">
              <span className="truncate">{shard.name}</span>
              <div className="text-muted-foreground flex flex-col text-xs tabular-nums">
                <Counts
                  label="Normal"
                  free={shard.owned_normal}
                  equipped={sum(shard.holders.map((holder) => holder.normal))}
                />
                <Counts
                  label="Tauforged"
                  free={shard.owned_tauforged}
                  equipped={sum(
                    shard.holders.map((holder) => holder.tauforged),
                  )}
                />
              </div>
              {shard.holders.length > 0 && <ShardHolders shard={shard} />}
            </div>
          </div>
        ))}
      </CardGrid>
    </Section>
  );
}
