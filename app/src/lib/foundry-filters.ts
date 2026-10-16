import { failsYesNo, passesYesNo, type YesNo } from "@/lib/filters";
import type { FoundryItem } from "@/types";

export const CATEGORIES = [
  ["all", "All"],
  ["warframe", "Warframe"],
  ["necramech", "Necramech"],
  ["primary", "Primary"],
  ["secondary", "Secondary"],
  ["melee", "Melee"],
  ["modular", "Modular"],
  ["arch", "Arch"],
  ["companion", "Companion"],
] as const;

export type Category = (typeof CATEGORIES)[number][0];

export const FILTERS = [
  { key: "prime", label: "Type", yes: "Prime", no: "Normal" },
  { key: "vaulted", label: "Vaulted", yes: "Yes", no: "No" },
  { key: "primeResurgence", label: "Prime Resurgence", yes: "Yes", no: "No" },
  { key: "owned", label: "Owned", yes: "Yes", no: "No" },
  { key: "mastered", label: "Mastered", yes: "Yes", no: "No" },
  { key: "ready", label: "Ready to build", yes: "Yes", no: "No" },
  {
    key: "enoughMastery",
    label: "Mastery rank requirement",
    yes: "Met",
    no: "Too low",
  },
  { key: "favourite", label: "Favourite", yes: "Yes", no: "No" },
  { key: "subsumed", label: "Subsumed", yes: "Yes", no: "No" },
  { key: "archonShards", label: "Archon Shards", yes: "Installed", no: "None" },
  { key: "incarnon", label: "Incarnon", yes: "Yes", no: "No" },
  {
    key: "usedCrafting",
    label: "Needed in other recipes",
    yes: "Yes",
    no: "No",
  },
] as const;

export type FilterKey = (typeof FILTERS)[number]["key"];
export type Filters = Partial<Record<FilterKey, YesNo>>;

type Flag = boolean | null;

const FLAGS: Record<FilterKey, (item: FoundryItem) => Flag> = {
  prime: (item) => item.prime !== null,
  vaulted: (item) =>
    item.prime === null ? null : item.prime.vault === "vaulted",
  primeResurgence: (item) =>
    item.prime === null ? null : item.prime.resurgence,
  owned: (item) => item.progress.owned,
  mastered: (item) => item.mastered,
  ready: (item) =>
    item.components.length > 0 ? item.progress.ready_to_build : null,
  enoughMastery: (item) =>
    item.mastery.required === null ? null : item.mastery.met,
  favourite: (item) => item.any_favourite,
  subsumed: (item) => (item.helminth === null ? null : item.helminth.subsumed),
  archonShards: (item) =>
    item.archon_shards === null ? null : item.archon_shards > 0,
  incarnon: (item) => item.incarnon,
  usedCrafting: (item) =>
    item.crafts_into === null ? null : item.crafts_into.length > 0,
};

const FILTER_KEYS = FILTERS.map((filter) => filter.key);

function byKey<T>(make: (key: FilterKey) => T): Record<FilterKey, T> {
  return Object.fromEntries(
    FILTER_KEYS.map((key) => [key, make(key)]),
  ) as Record<FilterKey, T>;
}

export interface Facet {
  yes: number;
  no: number;
  na: number;
}

function count(facet: Facet, value: Flag) {
  if (value === null) {
    facet.na += 1;
  } else if (value) {
    facet.yes += 1;
  } else {
    facet.no += 1;
  }
}

export function facets(
  items: FoundryItem[],
  filters: Filters,
): Record<FilterKey, Facet> {
  const result = byKey(() => ({ yes: 0, no: 0, na: 0 }));
  for (const item of items) {
    const flags = byKey((key) => FLAGS[key](item));
    const failing = FILTER_KEYS.filter((key) => {
      const mode = filters[key];
      return mode !== undefined && failsYesNo(mode, flags[key]);
    });
    if (failing.length > 1) {
      continue;
    }

    const counted = failing.length === 1 ? failing : FILTER_KEYS;
    for (const key of counted) {
      count(result[key], flags[key]);
    }
  }

  return result;
}

export function offered(facet: Facet): boolean {
  return facet.yes + facet.no > 0;
}

export function choice(
  facet: Facet,
  stored: YesNo | undefined,
): { value: YesNo | null; fixed: boolean } {
  if (!offered(facet)) {
    return { value: null, fixed: true };
  }

  if (facet.na === 0 && facet.no === 0) {
    return { value: "yes", fixed: true };
  }

  if (facet.na === 0 && facet.yes === 0) {
    return { value: "no", fixed: true };
  }

  return { value: stored ?? null, fixed: false };
}

function empty(facet: Facet, mode: YesNo): boolean {
  return mode === "yes" ? facet.yes === 0 : facet.no === 0;
}

export function prune(
  items: FoundryItem[],
  filters: Filters,
  keep?: FilterKey,
): Filters {
  const next = { ...filters };
  const base = facets(items, {});
  for (const key of FILTER_KEYS) {
    const mode = next[key];
    if (mode !== undefined && (!offered(base[key]) || empty(base[key], mode))) {
      delete next[key];
    }
  }

  const stale = (key: FilterKey) => {
    const mode = next[key];
    return mode !== undefined && key !== keep && empty(live[key], mode);
  };
  let live = facets(items, next);
  for (let key = FILTER_KEYS.find(stale); key; key = FILTER_KEYS.find(stale)) {
    delete next[key];
    live = facets(items, next);
  }

  for (const key of FILTER_KEYS) {
    const mode = next[key];
    if (mode !== undefined && key !== keep && choice(live[key], mode).fixed) {
      delete next[key];
    }
  }

  return Object.keys(next).length === Object.keys(filters).length
    ? filters
    : next;
}

export function withFilter(
  items: FoundryItem[],
  filters: Filters,
  key: FilterKey,
  mode: YesNo | null,
): Filters {
  const next = { ...filters };
  if (mode === null) {
    delete next[key];
  } else {
    next[key] = mode;
  }

  return prune(items, next, key);
}

function searchFoundry(item: FoundryItem, search: string): boolean {
  const normalizedSearch = search.trim().toLowerCase();
  if (!normalizedSearch) {
    return true;
  }

  if (item.type_name.toLowerCase() === normalizedSearch) {
    return true;
  }

  if (item.name.toLowerCase().includes(normalizedSearch)) {
    return true;
  }

  return item.components.some((component) =>
    component.name.toLowerCase().includes(normalizedSearch),
  );
}

export function foundryMatches(
  item: FoundryItem,
  search: string,
  filters: Filters,
): boolean {
  return (
    searchFoundry(item, search) &&
    passesYesNo(filters, (key) => FLAGS[key](item))
  );
}

export function inCategory(items: FoundryItem[], category: Category) {
  return category === "all"
    ? items
    : items.filter((item) => item.kind === category);
}
