import type { ArcaneRarity } from "@/components/game-icon";
import { passesYesNo } from "@/lib/filters";
import type {
  InventoryTabKey,
  SortableRow,
  TabFilters,
  YesNoKey,
} from "@/lib/inventory-filters";
import type {
  InventoryTab,
  MiscRow,
  ModRow,
  PartRow,
  RelicRow,
  SetRow,
  VaultStatus,
} from "@/types";

export interface Row extends SortableRow {
  key: string;
  uniqueName: string;
  favourite: boolean;
  orderPlaced: boolean;
  imageName: string | null;
  marketThumb: string | null;
  marketSlug: string;
  prime: boolean;
  vault: VaultStatus | null;
  itemOwned: boolean;
  mastered: boolean;
  setComplete: boolean | null;
  moreThanOne: boolean;
  rank: number | null;
  maxRank: number | null;
  platMaxRank: number | null;
  platIsFloor: boolean;
  buyPlat: number | null;
  rarity: ArcaneRarity | null;
  equippedIn: string[];
  subtitle: string | null;
  refinement: string | null;
  tier: string | null;
  set: SetRow | null;
}

function partRows(rows: PartRow[]): Row[] {
  return rows.map((row) => ({
    key: row.unique_name,
    uniqueName: row.unique_name,
    favourite: row.favourite,
    orderPlaced: row.order_placed,
    name: row.name,
    count: row.count,
    plat: row.prices.sell,
    ducats: row.prices.ducats,
    completion: row.set.complete ? 1 : 0,
    imageName: row.image_name,
    marketThumb: null,
    marketSlug: row.market_slug,
    prime: row.prime,
    vault: row.vault,
    itemOwned: row.item.built || row.item.mastered,
    mastered: row.item.mastered,
    setComplete: row.set.complete,
    moreThanOne: row.count > 1,
    rank: null,
    maxRank: null,
    buyPlat: row.prices.buy,
    platMaxRank: null,
    platIsFloor: false,
    rarity: null,
    equippedIn: [],
    subtitle: row.set.name,
    refinement: null,
    tier: null,
    set: null,
  }));
}

function setRows(rows: SetRow[]): Row[] {
  return rows.map((row) => ({
    key: row.unique_name,
    uniqueName: row.unique_name,
    favourite: row.favourite,
    orderPlaced: row.order_placed,
    name: row.set_name,
    count: row.count,
    plat: row.prices.sell,
    ducats: row.prices.ducats,
    completion: row.owned_parts / Math.max(row.total_parts, 1),
    imageName: row.image_name,
    marketThumb: null,
    marketSlug: row.market_slug,
    prime: row.set_name.includes("Prime"),
    vault: row.vault,
    itemOwned: row.item.built || row.item.mastered,
    mastered: row.item.mastered,
    setComplete: row.complete,
    moreThanOne: row.count > 1,
    rank: null,
    maxRank: null,
    buyPlat: row.prices.buy,
    platMaxRank: null,
    platIsFloor: false,
    rarity: null,
    equippedIn: [],
    subtitle: null,
    refinement: null,
    tier: null,
    set: row,
  }));
}

function modRows(rows: ModRow[]): Row[] {
  const copies = new Map<string, number>();
  for (const row of rows) {
    copies.set(row.unique_name, (copies.get(row.unique_name) ?? 0) + row.count);
  }
  return rows.map((row, index) => ({
    key: `${row.unique_name}-${row.rank ?? "u"}-${index}`,
    uniqueName: row.unique_name,
    favourite: row.favourite,
    orderPlaced: row.order_placed,
    name: row.name,
    count: row.count,
    plat: row.prices.sell,
    ducats: null,
    completion: 0,
    imageName: row.image_name,
    marketThumb: row.market_thumb,
    marketSlug: row.market_slug,
    prime: row.prime,
    vault: null,
    itemOwned: false,
    mastered: false,
    setComplete: null,
    moreThanOne: (copies.get(row.unique_name) ?? row.count) > 1,
    rank: row.rank,
    maxRank: row.max_rank,
    buyPlat: row.prices.buy,
    platMaxRank: row.prices.sell_max_rank,
    platIsFloor: row.prices.is_floor,
    rarity: row.rarity,
    equippedIn: row.equipped_in,
    subtitle: null,
    refinement: null,
    tier: null,
    set: null,
  }));
}

function relicRows(rows: RelicRow[]): Row[] {
  return rows.map((row) => ({
    key: row.unique_name,
    uniqueName: row.unique_name,
    favourite: row.favourite,
    orderPlaced: row.order_placed,
    name: row.relic,
    count: row.count,
    plat: row.plat,
    ducats: null,
    completion: 0,
    imageName: row.image_name,
    marketThumb: null,
    marketSlug: "",
    prime: false,
    vault: row.vault,
    itemOwned: false,
    mastered: false,
    setComplete: null,
    moreThanOne: row.count > 1,
    rank: null,
    maxRank: null,
    buyPlat: null,
    platMaxRank: null,
    platIsFloor: false,
    rarity: null,
    equippedIn: [],
    subtitle: null,
    refinement: row.refinement,
    tier: row.tier,
    set: null,
  }));
}

function miscRows(rows: MiscRow[]): Row[] {
  return rows.map((row) => ({
    key: row.unique_name,
    uniqueName: row.unique_name,
    favourite: row.favourite,
    orderPlaced: row.order_placed,
    name: row.name,
    count: row.count,
    plat: row.plat,
    ducats: row.ducats,
    completion: 0,
    imageName: row.image_name,
    marketThumb: null,
    marketSlug: row.market_slug,
    prime: row.name.includes("Prime"),
    vault: null,
    itemOwned: false,
    mastered: false,
    setComplete: null,
    moreThanOne: row.count > 1,
    rank: null,
    maxRank: null,
    buyPlat: null,
    platMaxRank: null,
    platIsFloor: false,
    rarity: null,
    equippedIn: [],
    subtitle: null,
    refinement: null,
    tier: null,
    set: null,
  }));
}

export function rowsFor(
  tab: InventoryTabKey,
  data: InventoryTab | null,
): Row[] {
  if (!data) {
    return [];
  }
  switch (tab) {
    case "parts":
      return partRows(data.parts);
    case "sets":
      return setRows(data.sets);
    case "mods":
      return modRows(data.mods);
    case "arcanes":
      return modRows(data.arcanes);
    case "relics":
      return relicRows(data.relics);
    default:
      return miscRows(data.misc);
  }
}

export function rowPlat(row: Row): number | null {
  return row.maxRank !== null && row.rank === row.maxRank
    ? (row.platMaxRank ?? row.plat)
    : row.plat;
}

export function equippedLabel(row: Row): string | null {
  if (row.equippedIn.length === 0) {
    return null;
  }
  const listedFirst = row.equippedIn.slice(0, 3);
  const rest = row.equippedIn.length - listedFirst.length;
  const listed = listedFirst.join(", ");
  return rest > 0 ? `${listed} and ${rest} more` : listed;
}

const TRAITS: Record<YesNoKey, (row: Row) => boolean> = {
  itemOwned: (row) => row.itemOwned,
  duplicates: (row) => row.moreThanOne,
  vaulted: (row) => row.vault === "vaulted",
  prime: (row) => row.prime,
  fullSet: (row) => row.setComplete === true,
  ranked: (row) => Boolean(row.rank),
  equipped: (row) => row.equippedIn.length > 0,
  orderPlaced: (row) => row.orderPlaced,
  favourite: (row) => row.favourite,
};

function matchesSearch(row: Row, search: string): boolean {
  if (!search) {
    return true;
  }
  const haystack = `${row.name} ${row.subtitle ?? ""} ${row.refinement ?? ""}`;
  return haystack.toLowerCase().includes(search.toLowerCase());
}

export function keeps(row: Row, filters: TabFilters): boolean {
  if (!matchesSearch(row, filters.search)) {
    return false;
  }
  if (filters.refinement !== null && row.refinement !== filters.refinement) {
    return false;
  }
  if (!passesYesNo(filters.yesNo, (key) => TRAITS[key](row))) {
    return false;
  }
  return filters.minPlat === null || (row.plat ?? 0) >= filters.minPlat;
}
