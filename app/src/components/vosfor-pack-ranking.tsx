import { useEffect, useMemo } from "react";
import { Figure, Figures } from "@/components/figure";
import {
  ArcaneImage,
  ItemImage,
  prefetchImages,
} from "@/components/item-image";
import { EmptyPanel } from "@/components/page";
import { Hint } from "@/components/ui/hint";
import { itemAt } from "@/lib/arrays";
import { counted, num, percent } from "@/lib/format";
import {
  type ArcaneCollectionEntry,
  arcaneCollectionEntries,
  missingCopies,
} from "@/lib/inventory-entries";
import { cn } from "@/lib/utils";
import type { InventoryTab, VosforPackRow } from "@/types";

interface VosforPackNeededEntry {
  arcane: ArcaneCollectionEntry;
  chancePerPack: number;
}

interface RankedVosforPack {
  pack: VosforPackRow;
  needed: VosforPackNeededEntry[];
}

function VosforPackNeededRow({
  arcane: { row, item },
  chancePerPack,
}: VosforPackNeededEntry) {
  return (
    <li className="flex items-center gap-2">
      <ArcaneImage
        imageName={item.image_name}
        rarity={row.rarity}
        size={28}
        alt=""
      />
      <span className="min-w-0 flex-1 truncate text-sm" title={item.name}>
        {item.name}
      </span>
      <span className="shrink-0 text-sm font-semibold">
        {missingCopies(row)}
      </span>
      <Hint
        as="span"
        className="w-12 shrink-0 text-right"
        title="Chance that a pack has at least one copy"
      >
        {percent(chancePerPack)}
      </Hint>
    </li>
  );
}

function VosforPackCard({ pack, needed }: RankedVosforPack) {
  const { completion } = pack;

  return (
    <li
      className={cn(
        "bg-card flex flex-col gap-3 rounded-xl border p-4",
        completion === null && "opacity-60",
      )}
    >
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
        <ItemImage
          imageName={pack.image_name}
          size={48}
          className="bg-transparent"
        />
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <span className="text-sm leading-tight font-semibold">
            {pack.name}
          </span>
          <Hint as="span">
            {completion === null
              ? `You have collected all ${num(pack.arcane_count)} arcanes of this pack`
              : `You need ${num(needed.length)} of the ${counted(pack.arcane_count, "available arcane")}`}
          </Hint>
        </div>
        {completion !== null && (
          <Figures>
            <Figure
              label="Needed per pack"
              title={`How many of the ${pack.draws} arcanes in a pack are ones you still need, on average`}
              value={`${pack.needed_per_pack.toFixed(2)} of ${num(pack.draws)}`}
            />
            <Figure
              label="Chance of a needed one"
              title="Chance that a pack has at least one arcane you still need"
              value={percent(pack.needed_chance)}
            />
            <Figure
              label="Packs needed"
              title="Average number of packs to open until you are no longer missing an arcane"
              value={`~${num(completion.packs)}`}
            />
            <Figure
              label="Vosfor needed"
              title="Total Vosfor paid to get all arcanes through this pack"
              value={`~${num(completion.vosfor)}`}
            />
          </Figures>
        )}
      </div>
      {completion !== null && (
        <ul className="grid gap-x-8 gap-y-1.5 border-t pt-3 @3xl:grid-cols-2 @7xl:grid-cols-3">
          {needed.map((entry) => (
            <VosforPackNeededRow
              key={entry.arcane.item.unique_name}
              {...entry}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

export function VosforPackRanking({ data }: { data: InventoryTab }) {
  const packs = useMemo(() => {
    const arcanes = arcaneCollectionEntries(data);
    return data.vosfor_packs.map(
      (pack): RankedVosforPack => ({
        pack,
        needed: pack.needed.map((needed) => ({
          arcane: itemAt(arcanes, needed.row),
          chancePerPack: needed.chance_per_pack,
        })),
      }),
    );
  }, [data]);

  useEffect(() => {
    prefetchImages(
      packs.flatMap(({ needed }) =>
        needed.map(({ arcane }) => arcane.item.image_name),
      ),
    );
  }, [packs]);

  if (packs.length === 0) {
    return (
      <EmptyPanel>
        The arcane collections couldn't be loaded with the game data. Merframe
        tries again on the next start.
      </EmptyPanel>
    );
  }

  return (
    <ol className="flex flex-col gap-3">
      {packs.map((rankedPack) => (
        <VosforPackCard key={rankedPack.pack.name} {...rankedPack} />
      ))}
    </ol>
  );
}
