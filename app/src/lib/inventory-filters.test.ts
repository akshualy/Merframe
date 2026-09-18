import { describe, expect, it } from "vitest";
import {
  compareRows,
  descending,
  refinementsFor,
  type SortableRow,
  type TabFilters,
  yesNoFiltersFor,
} from "@/lib/inventory-filters";

const filters: TabFilters = {
  search: "",
  ordering: "name",
  preferHighest: true,
  yesNo: {},
  minPlat: null,
  refinement: null,
};

function row(partial: Partial<SortableRow> & { name: string }): SortableRow {
  return {
    count: 1,
    plat: null,
    ducats: null,
    completion: 0,
    refinement: null,
    ...partial,
  };
}

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

describe("compareRows", () => {
  const alternox = row({
    name: "Alternox Prime Blueprint",
    plat: 168.25,
    ducats: 100,
  });
  const kompressa = row({
    name: "Kompressa Prime Receiver",
    plat: 25,
    ducats: 45,
  });
  const unpriced = row({ name: "Forma Blueprint", plat: null, ducats: 15 });

  it("name ascending", () => {
    expect(
      compareRows(alternox, kompressa, "name", false, "parts"),
    ).toBeLessThan(0);
  });

  it("plat descending", () => {
    expect(
      compareRows(alternox, kompressa, "plat", true, "parts"),
    ).toBeLessThan(0);
  });

  it("ducats per plat treats unpriced parts as nearly free", () => {
    expect(
      compareRows(unpriced, alternox, "ducatsPerPlat", true, "parts"),
    ).toBeLessThan(0);
  });

  it("set progress sorts sets by completion and parts by name", () => {
    const half = row({ name: "Kompressa Prime Set", completion: 0.5 });
    const full = row({ name: "Alternox Prime Set", completion: 1 });
    expect(
      compareRows(half, full, "setProgress", true, "sets"),
    ).toBeGreaterThan(0);
    expect(
      compareRows(half, full, "setProgress", false, "parts"),
    ).toBeGreaterThan(0);
  });

  it("refinement follows the refinement order", () => {
    const intact = row({ name: "Lith G12", refinement: "Intact" });
    const radiant = row({ name: "Lith G12", refinement: "Radiant" });
    expect(
      compareRows(intact, radiant, "refinement", false, "relics"),
    ).toBeLessThan(0);
  });

  it("ties break on the name", () => {
    const a = row({ name: "Axi A20", count: 6 });
    const b = row({ name: "Lith B11", count: 6 });
    expect(compareRows(a, b, "count", true, "relics")).toBeLessThan(0);
  });
});
