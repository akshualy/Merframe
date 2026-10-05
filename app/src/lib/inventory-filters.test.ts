import { describe, expect, it } from "vitest";
import {
  entriesFor,
  equippedLabel,
  type InventoryEntry,
  inventoryFlags,
  totalPlat,
  vault,
} from "@/lib/inventory-entries";
import {
  compareEntries,
  descending,
  keeps,
  refinementsFor,
  type TabFilters,
  yesNoFiltersFor,
} from "@/lib/inventory-filters";
import type {
  InventoryTab,
  ItemSummary,
  MiscRow,
  ModHolder,
  ModRow,
  PartRow,
  RelicRow,
} from "@/types";

const filters: TabFilters = {
  search: "",
  ordering: "name",
  preferHighest: true,
  yesNo: {},
  minPlat: null,
  refinement: null,
};

function summary(name: string, extra: Partial<ItemSummary> = {}): ItemSummary {
  return {
    unique_name: `/Lotus/${name.replaceAll(" ", "")}`,
    name,
    image_name: null,
    market_slug: name.toLowerCase().replaceAll(" ", "_"),
    prime: name.includes("Prime"),
    vault: null,
    ...extra,
  };
}

function part(
  name: string,
  extra: { plat?: number | null; ducats?: number | null } = {},
): InventoryEntry {
  const row: PartRow = {
    item: 0,
    count: 1,
    prices: {
      sell: extra.plat ?? null,
      buy: null,
      ducats: extra.ducats ?? null,
    },
    set: { item: 1, complete: false },
    status: { built: false, mastered: false },
  };
  return { kind: "part", row, item: summary(name), set: summary("Set") };
}

function set(name: string, owned: number, total: number): InventoryEntry {
  return {
    kind: "set",
    row: {
      item: 0,
      owned_parts: owned,
      total_parts: total,
      count: 0,
      complete: owned === total,
      status: { built: false, mastered: false },
      prices: { sell: null, buy: null, ducats: null },
      components: [],
    },
    item: summary(name),
    parts: [],
  };
}

function relic(
  name: string,
  refinement: RelicRow["refinement"],
  count = 1,
): InventoryEntry {
  const row: RelicRow = {
    item: 0,
    tier: "Lith",
    refinement,
    count,
    plat: null,
  };
  return { kind: "relic", row, item: summary(name, { vault: "vaulted" }) };
}

function holder(item_id: string, name: string): ModHolder {
  return {
    item_id,
    name,
    custom_name: null,
    image_name: null,
    rank: 30,
    takes_orokin_reactor: true,
    orokin_upgrade: false,
    exilus_adapter: false,
    configs: [0],
    forma: 0,
    archon_shards: 0,
  };
}

function modRow(
  item: number,
  rank: number | null,
  count: number,
  equipped_in: number[] = [],
): ModRow {
  return {
    item,
    count,
    rank,
    max_rank: 10,
    prices: { sell: 20, sell_max_rank: 90, is_floor: false, buy: null },
    rarity: null,
    equipped_in,
  };
}

const misc: MiscRow = {
  item: 4,
  count: 1,
  ducats: null,
  plat: 3,
  market_subtype: null,
  stars: null,
};

const tab: InventoryTab = {
  items: [
    summary("Braton Prime Barrel", { vault: "available" }),
    summary("Braton Prime"),
    summary("Serration"),
    summary("Arcane Energize"),
    summary("Kavasa Prime Band", { vault: "vaulted" }),
  ],
  holders: [holder("a", "Trinity Prime"), holder("b", "Rhino")],
  favourites: [2],
  selling: [1],
  buying: [],
  parts: [
    {
      item: 0,
      count: 2,
      prices: { sell: 8, buy: 5, ducats: 45 },
      set: { item: 1, complete: true },
      status: { built: true, mastered: false },
    },
  ],
  mods: [modRow(2, 0, 1), modRow(2, 10, 1, [1, 0])],
  arcanes: [modRow(3, 0, 1)],
  relics: [],
  misc: [misc],
  sets: [],
};

describe("yesNoFiltersFor", () => {
  it("set filters only on parts and sets", () => {
    const keys = (tab: "parts" | "relics") =>
      yesNoFiltersFor(tab).map((group) => group.key);
    expect(keys("parts")).toContain("fullSet");
    expect(keys("relics")).not.toContain("fullSet");
  });

  it("vaulted on relics, ranked on mods", () => {
    expect(yesNoFiltersFor("relics").map((g) => g.key)).toContain("vaulted");
    expect(yesNoFiltersFor("mods").map((g) => g.key)).toContain("ranked");
    expect(yesNoFiltersFor("misc").map((g) => g.key)).not.toContain("ranked");
  });
});

describe("refinementsFor", () => {
  it("relics list the four refinements", () => {
    expect(refinementsFor("relics")).toEqual([
      "Intact",
      "Exceptional",
      "Flawless",
      "Radiant",
    ]);
  });

  it("nothing on other tabs", () => {
    expect(refinementsFor("parts")).toEqual([]);
  });
});

describe("descending", () => {
  it("name prefers A to Z", () => {
    expect(descending(filters)).toBe(false);
  });

  it("values prefer highest first", () => {
    expect(descending({ ...filters, ordering: "plat" })).toBe(true);
    expect(
      descending({ ...filters, ordering: "plat", preferHighest: false }),
    ).toBe(false);
  });
});

describe("entriesFor", () => {
  it("joins each row with its item and its set", () => {
    const [barrel] = entriesFor("parts", tab);
    expect(barrel?.item.name).toBe("Braton Prime Barrel");
    expect(barrel?.kind === "part" && barrel.set.name).toBe("Braton Prime");
  });

  it("counts a mod across its ranks", () => {
    const mods = entriesFor("mods", tab);
    expect(mods).toHaveLength(2);
    expect(mods.every((entry) => entry.kind === "upgrade")).toBe(true);
    expect(
      mods.map((entry) => entry.kind === "upgrade" && entry.moreThanOne),
    ).toEqual([true, true]);
    const [arcane] = entriesFor("arcanes", tab);
    expect(arcane?.kind === "upgrade" && arcane.moreThanOne).toBe(false);
  });

  it("nothing without data", () => {
    expect(entriesFor("misc", null)).toEqual([]);
  });
});

describe("keeps", () => {
  const flags = inventoryFlags(tab);

  it("searches the set name of a part", () => {
    const [barrel] = entriesFor("parts", tab);
    if (!barrel) {
      throw new Error("no part");
    }
    expect(keeps(barrel, { ...filters, search: "braton prime" }, flags)).toBe(
      true,
    );
    expect(keeps(barrel, { ...filters, search: "lato" }, flags)).toBe(false);
  });

  it("reads favourites and orders from the item sets", () => {
    const [serration] = entriesFor("mods", tab);
    const [barrel] = entriesFor("parts", tab);
    if (!serration || !barrel) {
      throw new Error("no rows");
    }
    const favourite = { ...filters, yesNo: { favourite: "yes" as const } };
    expect(keeps(serration, favourite, flags)).toBe(true);
    expect(keeps(barrel, favourite, flags)).toBe(false);
    const ordered = { ...filters, yesNo: { orderPlaced: "yes" as const } };
    expect(keeps(barrel, ordered, flags)).toBe(false);
    expect(keeps(barrel, { ...filters, minPlat: 5 }, flags)).toBe(true);
    expect(keeps(barrel, { ...filters, minPlat: 10 }, flags)).toBe(false);
  });

  it("filters crafted parts and full sets", () => {
    const [barrel] = entriesFor("parts", tab);
    if (!barrel) {
      throw new Error("no part");
    }
    const yes = { itemOwned: "yes" as const, fullSet: "yes" as const };
    expect(keeps(barrel, { ...filters, yesNo: yes }, flags)).toBe(true);
  });
});

describe("totalPlat", () => {
  it("prices a maxed mod at its max rank", () => {
    const [unranked, maxed] = entriesFor("mods", tab);
    if (!unranked || !maxed) {
      throw new Error("no mods");
    }
    expect(totalPlat(unranked)).toBe(20);
    expect(totalPlat(maxed)).toBe(90);
  });
});

describe("vault", () => {
  it("only parts, sets and relics show a vault state", () => {
    const [kavasa] = entriesFor("misc", tab);
    const [barrel] = entriesFor("parts", tab);
    if (!kavasa || !barrel) {
      throw new Error("no rows");
    }
    expect(vault(kavasa)).toBeNull();
    expect(vault(barrel)).toBe("available");
  });
});

describe("compareEntries", () => {
  const alternox = part("Alternox Prime Blueprint", {
    plat: 168.25,
    ducats: 100,
  });
  const kompressa = part("Kompressa Prime Receiver", { plat: 25, ducats: 45 });
  const unpriced = part("Forma Blueprint", { plat: null, ducats: 15 });

  it("name ascending", () => {
    expect(
      compareEntries(alternox, kompressa, "name", false, "parts"),
    ).toBeLessThan(0);
  });

  it("plat descending", () => {
    expect(
      compareEntries(alternox, kompressa, "plat", true, "parts"),
    ).toBeLessThan(0);
  });

  it("ducats per plat treats unpriced parts as nearly free", () => {
    expect(
      compareEntries(unpriced, alternox, "ducatsPerPlat", true, "parts"),
    ).toBeLessThan(0);
  });

  it("set progress sorts sets by completion and parts by name", () => {
    const half = set("Kompressa Prime Set", 1, 2);
    const full = set("Alternox Prime Set", 2, 2);
    expect(
      compareEntries(half, full, "setProgress", true, "sets"),
    ).toBeGreaterThan(0);
    expect(
      compareEntries(half, full, "setProgress", false, "parts"),
    ).toBeGreaterThan(0);
  });

  it("refinement follows the refinement order", () => {
    const intact = relic("Lith G12", "Intact");
    const radiant = relic("Lith G12", "Radiant");
    expect(
      compareEntries(intact, radiant, "refinement", false, "relics"),
    ).toBeLessThan(0);
  });

  it("ties break on the name", () => {
    const a = relic("Axi A20", "Intact", 6);
    const b = relic("Lith B11", "Intact", 6);
    expect(compareEntries(a, b, "count", true, "relics")).toBeLessThan(0);
  });
});

describe("equipped holders", () => {
  it("joins holders and filters on them", () => {
    const [spare, equipped] = entriesFor("mods", tab);
    if (!spare || !equipped) {
      throw new Error("no mods");
    }
    const flags = inventoryFlags(tab);
    const inLoadout = { ...filters, yesNo: { equipped: "yes" as const } };
    expect(keeps(spare, inLoadout, flags)).toBe(false);
    expect(keeps(equipped, inLoadout, flags)).toBe(true);
    expect(equipped.kind === "upgrade" && equippedLabel(equipped.holders)).toBe(
      "Rhino, Trinity Prime",
    );
  });

  it("lists five names and counts the rest", () => {
    const many = ["A", "B", "C", "D", "E", "F", "G"].map((name) =>
      holder(name, name),
    );
    expect(equippedLabel(many)).toBe("A, B, C, D, E and 2 more");
    expect(equippedLabel([])).toBeNull();
  });
});
