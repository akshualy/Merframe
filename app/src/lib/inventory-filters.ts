import type { YesNo } from "@/lib/filters";
import { REFINEMENTS } from "@/lib/relics";
import { readStoredJson, writeStoredJson } from "@/lib/storage";

export const INVENTORY_TABS = [
  "parts",
  "relics",
  "mods",
  "arcanes",
  "misc",
  "sets",
] as const;

export type InventoryTabKey = (typeof INVENTORY_TABS)[number];

export const ORDERINGS = [
  { value: "name", label: "Name" },
  { value: "plat", label: "Platinum" },
  { value: "ducats", label: "Ducats" },
  { value: "count", label: "Count" },
  { value: "ducatsPerPlat", label: "Ducats per plat" },
  { value: "setProgress", label: "Set progress" },
  { value: "refinement", label: "Refinement" },
] as const;

export type Ordering = (typeof ORDERINGS)[number]["value"];

const EVERY_TAB: readonly InventoryTabKey[] = INVENTORY_TABS;
const SET_TABS: readonly InventoryTabKey[] = ["parts", "sets"];
const VAULT_TABS: readonly InventoryTabKey[] = ["parts", "sets", "relics"];
const RANK_TABS: readonly InventoryTabKey[] = ["mods", "arcanes"];
const MOD_TABS: readonly InventoryTabKey[] = ["mods"];
const PRIME_TABS: readonly InventoryTabKey[] = [
  "parts",
  "sets",
  "mods",
  "arcanes",
  "misc",
];

export const YES_NO_FILTERS = [
  {
    key: "itemOwned",
    label: "Item crafted",
    yes: "Built or mastered",
    no: "Neither",
    tabs: SET_TABS,
  },
  {
    key: "duplicates",
    label: "Duplicates",
    yes: "Yes",
    no: "No",
    tabs: EVERY_TAB,
  },
  {
    key: "vaulted",
    label: "Vaulted",
    yes: "Yes",
    no: "No",
    tabs: VAULT_TABS,
  },
  {
    key: "prime",
    label: "Prime",
    yes: "Prime",
    no: "Non-prime",
    tabs: PRIME_TABS,
  },
  {
    key: "fullSet",
    label: "Full set",
    yes: "Yes",
    no: "No",
    tabs: SET_TABS,
  },
  {
    key: "ranked",
    label: "Ranked",
    yes: "Yes",
    no: "No",
    tabs: RANK_TABS,
  },
  {
    key: "equipped",
    label: "Equipped",
    yes: "In a loadout",
    no: "Spare",
    tabs: MOD_TABS,
  },
  {
    key: "orderPlaced",
    label: "Order placed",
    yes: "Yes",
    no: "No",
    tabs: EVERY_TAB,
  },
  {
    key: "favourite",
    label: "Favourite",
    yes: "Favourites",
    no: "Not favourites",
    tabs: EVERY_TAB,
  },
] as const;

export type YesNoGroup = (typeof YES_NO_FILTERS)[number];

export function yesNoFiltersFor(tab: InventoryTabKey): YesNoGroup[] {
  return YES_NO_FILTERS.filter((group) => group.tabs.includes(tab));
}

const REFINEMENT_TABS: readonly InventoryTabKey[] = ["relics"];
const REFINEMENT_ORDER: readonly string[] = REFINEMENTS;

export function refinementsFor(tab: InventoryTabKey): readonly string[] {
  return REFINEMENT_TABS.includes(tab) ? REFINEMENT_ORDER : [];
}

export type YesNoKey = YesNoGroup["key"];
export type YesNoFilters = Partial<Record<YesNoKey, YesNo>>;

export interface TabFilters {
  search: string;
  ordering: Ordering;
  preferHighest: boolean;
  yesNo: YesNoFilters;
  minPlat: number | null;
  refinement: string | null;
}

const DEFAULT_FILTERS: TabFilters = {
  search: "",
  ordering: "name",
  preferHighest: true,
  yesNo: {},
  minPlat: null,
  refinement: null,
};

const UNPRICED_PLAT = 0.0001;

export function descending(filters: TabFilters): boolean {
  return filters.ordering === "name"
    ? !filters.preferHighest
    : filters.preferHighest;
}

const STORAGE_KEY = "merframe.inventory.view";

function knownYesNo(stored: YesNoFilters, tab: InventoryTabKey): YesNoFilters {
  const kept: YesNoFilters = {};
  for (const group of yesNoFiltersFor(tab)) {
    const choice = stored[group.key];
    if (choice === "yes" || choice === "no") {
      kept[group.key] = choice;
    }
  }
  return kept;
}

function knownOrdering(stored: Ordering | undefined): Ordering {
  const known = ORDERINGS.find((option) => option.value === stored);
  return known ? known.value : DEFAULT_FILTERS.ordering;
}

function knownRefinement(
  stored: string | null | undefined,
  tab: InventoryTabKey,
): string | null {
  return stored && refinementsFor(tab).includes(stored) ? stored : null;
}

export function loadFilters(tab: InventoryTabKey): TabFilters {
  const parsed = readStoredJson<Partial<TabFilters>>(`${STORAGE_KEY}.${tab}`);
  if (!parsed) {
    return { ...DEFAULT_FILTERS };
  }
  return {
    ...DEFAULT_FILTERS,
    ...parsed,
    ordering: knownOrdering(parsed.ordering),
    yesNo: knownYesNo(parsed.yesNo ?? {}, tab),
    refinement: knownRefinement(parsed.refinement, tab),
  };
}

export function saveFilters(tab: InventoryTabKey, filters: TabFilters) {
  writeStoredJson(`${STORAGE_KEY}.${tab}`, filters);
}

export interface SortableRow {
  name: string;
  count: number;
  plat: number | null;
  ducats: number | null;
  completion: number;
  refinement: string | null;
}

function orderValue(
  row: SortableRow,
  ordering: Ordering,
  tab: InventoryTabKey,
): number | string {
  switch (ordering) {
    case "plat":
      return row.plat ?? 0;
    case "ducats":
      return row.ducats ?? 0;
    case "count":
      return row.count;
    case "ducatsPerPlat": {
      if (tab === "sets") {
        return (row.plat ?? 0) / (row.ducats ?? 0);
      }
      const plat = row.plat ?? 0;
      return (row.ducats ?? 0) / (plat > 0 ? plat : UNPRICED_PLAT);
    }
    case "setProgress":
      return tab === "sets" ? row.completion : row.name;
    case "refinement":
      return row.refinement === null
        ? row.name
        : REFINEMENT_ORDER.indexOf(row.refinement);
    default:
      return row.name;
  }
}

export function compareRows(
  left: SortableRow,
  right: SortableRow,
  ordering: Ordering,
  desc: boolean,
  tab: InventoryTabKey,
): number {
  const a = orderValue(left, ordering, tab);
  const b = orderValue(right, ordering, tab);
  let primary = 0;
  if (typeof a === "string" || typeof b === "string") {
    primary = String(a).localeCompare(String(b));
  } else if (Number.isFinite(a - b)) {
    primary = a - b;
  } else {
    primary = a === b ? 0 : a < b ? -1 : 1;
  }
  if (desc) {
    primary = -primary;
  }
  if (primary !== 0) {
    return primary;
  }
  return left.name.localeCompare(right.name);
}
