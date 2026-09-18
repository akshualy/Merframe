import { passesYesNo, type YesNo } from "@/lib/filters";
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

const EVERY_CATEGORY: readonly Category[] = CATEGORIES.map(([key]) => key);
const WARFRAMES_ONLY: readonly Category[] = ["warframe"];

export const FILTERS = [
  {
    key: "prime",
    label: "Type",
    yes: "Prime",
    no: "Normal",
    categories: EVERY_CATEGORY,
  },
  {
    key: "mastered",
    label: "Mastery",
    yes: "Mastered",
    no: "Unmastered",
    categories: EVERY_CATEGORY,
  },
  {
    key: "owned",
    label: "Owned",
    yes: "Yes",
    no: "No",
    categories: EVERY_CATEGORY,
  },
  {
    key: "vaulted",
    label: "Vaulted",
    yes: "Yes",
    no: "No",
    categories: EVERY_CATEGORY,
  },
  {
    key: "ready",
    label: "Ready to build",
    yes: "Yes",
    no: "No",
    categories: EVERY_CATEGORY,
  },
  {
    key: "enoughMastery",
    label: "Mastery rank",
    yes: "Enough",
    no: "Too low",
    categories: EVERY_CATEGORY,
  },
  {
    key: "primeResurgence",
    label: "Prime Resurgence",
    yes: "Yes",
    no: "No",
    categories: EVERY_CATEGORY,
  },
  {
    key: "incarnon",
    label: "Incarnon",
    yes: "Yes",
    no: "No",
    categories: EVERY_CATEGORY,
  },
  {
    key: "usedCrafting",
    label: "Crafting stock",
    yes: "Another recipe needs it",
    no: "Nothing needs it",
    categories: EVERY_CATEGORY,
  },
  {
    key: "subsumed",
    label: "Helminth",
    yes: "Subsumed",
    no: "Not subsumed",
    categories: WARFRAMES_ONLY,
  },
  {
    key: "archonShards",
    label: "Archon shards",
    yes: "Installed",
    no: "None",
    categories: WARFRAMES_ONLY,
  },
  {
    key: "favourite",
    label: "Favourite",
    yes: "Favourites",
    no: "Not favourites",
    categories: EVERY_CATEGORY,
  },
] as const;

export type FilterGroup = (typeof FILTERS)[number];

export function filtersFor(category: Category): FilterGroup[] {
  return FILTERS.filter((group) => group.categories.includes(category));
}

export type FilterKey = FilterGroup["key"];
export type Filters = Partial<Record<FilterKey, YesNo>>;

function flag(item: FoundryItem, key: FilterKey): boolean {
  switch (key) {
    case "prime":
      return item.prime !== null;
    case "mastered":
      return item.mastered;
    case "owned":
      return item.progress.owned;
    case "vaulted":
      return item.prime?.vault === "vaulted";
    case "enoughMastery":
      return item.mastery.met;
    case "primeResurgence":
      return item.prime?.resurgence ?? false;
    case "incarnon":
      return item.incarnon;
    case "subsumed":
      return item.helminth?.subsumed ?? false;
    case "archonShards":
      return item.archon_shards > 0;
    case "usedCrafting":
      return item.crafts_into.length > 0;
    case "favourite":
      return (
        item.favourite ||
        item.components.some((component) => component.favourite)
      );
    default:
      return item.progress.ready_to_build;
  }
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
    passesYesNo(filters, (key) => flag(item, key))
  );
}

export function scoped(filters: Filters, category: Category): Filters {
  const kept: Filters = {};
  for (const group of filtersFor(category)) {
    const mode = filters[group.key];
    if (mode) {
      kept[group.key] = mode;
    }
  }
  return kept;
}
