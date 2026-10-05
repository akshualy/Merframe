import { itemAt } from "@/lib/arrays";
import type { InventoryTabKey } from "@/lib/inventory-filters";
import type {
  InventoryTab,
  ItemSummary,
  MiscRow,
  ModHolder,
  ModRow,
  PartRow,
  RelicRow,
  SetComponent,
  SetRow,
  VaultStatus,
} from "@/types";

export interface SetPart {
  component: SetComponent;
  item: ItemSummary;
}

export type InventoryEntry =
  | { kind: "part"; row: PartRow; item: ItemSummary; set: ItemSummary }
  | { kind: "set"; row: SetRow; item: ItemSummary; parts: SetPart[] }
  | {
      kind: "upgrade";
      row: ModRow;
      item: ItemSummary;
      holders: ModHolder[];
      moreThanOne: boolean;
    }
  | { kind: "relic"; row: RelicRow; item: ItemSummary }
  | { kind: "misc"; row: MiscRow; item: ItemSummary };

export interface InventoryFlags {
  favourites: ReadonlySet<number>;
  selling: ReadonlySet<number>;
  buying: ReadonlySet<number>;
}

export function inventoryFlags(data: InventoryTab | null): InventoryFlags {
  return {
    favourites: new Set(data?.favourites),
    selling: new Set(data?.selling),
    buying: new Set(data?.buying),
  };
}

function upgradeEntries(
  rows: ModRow[],
  { items, holders }: InventoryTab,
): InventoryEntry[] {
  const copies = new Map<number, number>();
  for (const row of rows) {
    copies.set(row.item, (copies.get(row.item) ?? 0) + row.count);
  }
  return rows.map((row) => ({
    kind: "upgrade",
    row,
    item: itemAt(items, row.item),
    holders: row.equipped_in.map((holder) => itemAt(holders, holder)),
    moreThanOne: (copies.get(row.item) ?? row.count) > 1,
  }));
}

export function entriesFor(
  tab: InventoryTabKey,
  data: InventoryTab | null,
): InventoryEntry[] {
  if (!data) {
    return [];
  }
  const { items } = data;
  switch (tab) {
    case "parts":
      return data.parts.map((row) => ({
        kind: "part",
        row,
        item: itemAt(items, row.item),
        set: itemAt(items, row.set.item),
      }));
    case "sets":
      return data.sets.map((row) => ({
        kind: "set",
        row,
        item: itemAt(items, row.item),
        parts: row.components.map((component) => ({
          component,
          item: itemAt(items, component.item),
        })),
      }));
    case "mods":
      return upgradeEntries(data.mods, data);
    case "arcanes":
      return upgradeEntries(data.arcanes, data);
    case "relics":
      return data.relics.map((row) => ({
        kind: "relic",
        row,
        item: itemAt(items, row.item),
      }));
    case "misc":
      return data.misc.map((row) => ({
        kind: "misc",
        row,
        item: itemAt(items, row.item),
      }));
  }
}

export function entryKey(entry: InventoryEntry): string {
  switch (entry.kind) {
    case "upgrade":
      return `${entry.item.unique_name}-${entry.row.rank ?? "u"}`;
    case "misc":
      return entry.row.stars
        ? `${entry.item.unique_name}-${entry.row.stars.amber_filled}-${entry.row.stars.cyan_filled}`
        : entry.item.unique_name;
    default:
      return entry.item.unique_name;
  }
}

export function sellPlat(entry: InventoryEntry): number | null {
  switch (entry.kind) {
    case "relic":
    case "misc":
      return entry.row.plat;
    default:
      return entry.row.prices.sell;
  }
}

export function buyPlat(entry: InventoryEntry): number | null {
  switch (entry.kind) {
    case "relic":
    case "misc":
      return null;
    default:
      return entry.row.prices.buy;
  }
}

export function ducats(entry: InventoryEntry): number | null {
  switch (entry.kind) {
    case "part":
    case "set":
      return entry.row.prices.ducats;
    case "misc":
      return entry.row.ducats;
    default:
      return null;
  }
}

export function rank(entry: InventoryEntry): number | null {
  return entry.kind === "upgrade" ? entry.row.rank : null;
}

export function refinement(entry: InventoryEntry): string | null {
  return entry.kind === "relic" ? entry.row.refinement : null;
}

export function completion(entry: InventoryEntry): number {
  switch (entry.kind) {
    case "part":
      return entry.row.set.complete ? 1 : 0;
    case "set":
      return entry.row.owned_parts / Math.max(entry.row.total_parts, 1);
    default:
      return 0;
  }
}

export function setComplete(entry: InventoryEntry): boolean | null {
  switch (entry.kind) {
    case "part":
      return entry.row.set.complete;
    case "set":
      return entry.row.complete;
    default:
      return null;
  }
}

export function crafted(entry: InventoryEntry): boolean {
  return (
    (entry.kind === "part" || entry.kind === "set") &&
    (entry.row.status.built || entry.row.status.mastered)
  );
}

export function vault(entry: InventoryEntry): VaultStatus | null {
  return entry.kind === "upgrade" || entry.kind === "misc"
    ? null
    : entry.item.vault;
}

export function equippedIn(entry: InventoryEntry): ModHolder[] {
  return entry.kind === "upgrade" ? entry.holders : [];
}

export function totalPlat(entry: InventoryEntry): number | null {
  if (entry.kind !== "upgrade") {
    return sellPlat(entry);
  }
  const { row } = entry;
  return row.max_rank !== null && row.rank === row.max_rank
    ? (row.prices.sell_max_rank ?? row.prices.sell)
    : row.prices.sell;
}

export function hasOrder(
  entry: InventoryEntry,
  flags: InventoryFlags,
): boolean {
  return flags.selling.has(entry.row.item) || flags.buying.has(entry.row.item);
}

export function equippedLabel(holders: ModHolder[]): string | null {
  if (holders.length === 0) {
    return null;
  }
  const names = [...new Set(holders.map((holder) => holder.name))];
  const listedFirst = names.slice(0, 5);
  const rest = names.length - listedFirst.length;
  const listed = listedFirst.join(", ");
  return rest > 0 ? `${listed} and ${rest} more` : listed;
}
