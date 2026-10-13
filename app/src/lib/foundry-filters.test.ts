import { describe, expect, it } from "vitest";
import {
  choice,
  facets,
  foundryMatches,
  inCategory,
  offered,
  prune,
  withFilter,
} from "@/lib/foundry-filters";
import type { FoundryItem } from "@/types";
import foundry from "./foundry-filters.test-data.json";

const items = foundry.items as FoundryItem[];
const byName = (name: string) => {
  const item = items.find((entry) => entry.name === name);
  if (!item) {
    throw new Error(`${name} missing from the test data`);
  }

  return item;
};

const warframes = inCategory(items, "warframe");
const primaries = inCategory(items, "primary");

describe("foundryMatches", () => {
  const acceltra = byName("Acceltra");

  it("everything with an empty search", () => {
    expect(foundryMatches(acceltra, "  ", {})).toBe(true);
  });

  it("search by name", () => {
    expect(foundryMatches(acceltra, "accel", {})).toBe(true);
    expect(foundryMatches(acceltra, "braton", {})).toBe(false);
  });

  it("search by exact type name", () => {
    expect(foundryMatches(acceltra, "rifle", {})).toBe(true);
    expect(foundryMatches(acceltra, "rifl", {})).toBe(false);
  });

  it("search by component name", () => {
    expect(foundryMatches(acceltra, "hexenon", {})).toBe(true);
  });

  it("prime filter", () => {
    expect(foundryMatches(acceltra, "", { prime: "no" })).toBe(true);
    expect(foundryMatches(acceltra, "", { prime: "yes" })).toBe(false);
  });

  it("mastered and ready flags", () => {
    expect(
      foundryMatches(acceltra, "", { mastered: "yes", ready: "yes" }),
    ).toBe(true);
    expect(foundryMatches(acceltra, "", { owned: "yes" })).toBe(false);
  });

  it("prime flags under both values", () => {
    const nova = byName("Nova");
    expect(foundryMatches(nova, "", { vaulted: "yes" })).toBe(false);
    expect(foundryMatches(nova, "", { vaulted: "no" })).toBe(false);
    expect(foundryMatches(byName("Mesa Prime"), "", { vaulted: "no" })).toBe(
      true,
    );
  });

  it("helminth flags under both values", () => {
    const nova = byName("Nova Prime");
    expect(foundryMatches(nova, "", { subsumed: "yes" })).toBe(false);
    expect(foundryMatches(nova, "", { subsumed: "no" })).toBe(false);
  });

  it("favourite from components", () => {
    expect(
      foundryMatches(byName("Braton Prime"), "", { favourite: "yes" }),
    ).toBe(true);
  });
});

describe("facets", () => {
  it("counts per filter", () => {
    const counts = facets(warframes, {});
    expect(counts.prime).toEqual({ yes: 2, no: 2, na: 0 });
    expect(counts.vaulted).toEqual({ yes: 1, no: 1, na: 2 });
    expect(counts.subsumed).toEqual({ yes: 1, no: 1, na: 2 });
    expect(counts.incarnon).toEqual({ yes: 0, no: 0, na: 4 });
    expect(counts.enoughMastery).toEqual({ yes: 4, no: 0, na: 0 });
  });

  it("own selection left out", () => {
    const counts = facets(warframes, { prime: "yes" });
    expect(counts.prime).toEqual({ yes: 2, no: 2, na: 0 });
    expect(counts.subsumed).toEqual({ yes: 0, no: 0, na: 2 });
    expect(counts.vaulted).toEqual({ yes: 1, no: 1, na: 0 });
  });

  it("other selections applied", () => {
    const counts = facets(warframes, { prime: "no", owned: "yes" });
    expect(counts.subsumed).toEqual({ yes: 1, no: 1, na: 0 });
    expect(counts.mastered).toEqual({ yes: 1, no: 1, na: 0 });
  });
});

describe("choice", () => {
  it("free with both values", () => {
    expect(choice({ yes: 2, no: 3, na: 0 }, "no")).toEqual({
      value: "no",
      fixed: false,
    });
    expect(choice({ yes: 2, no: 3, na: 0 }, undefined)).toEqual({
      value: null,
      fixed: false,
    });
  });

  it("fixed on one value", () => {
    expect(choice({ yes: 0, no: 3, na: 0 }, "yes")).toEqual({
      value: "no",
      fixed: true,
    });
    expect(choice({ yes: 2, no: 0, na: 0 }, undefined)).toEqual({
      value: "yes",
      fixed: true,
    });
  });

  it("free with not applicable rows", () => {
    expect(choice({ yes: 2, no: 0, na: 5 }, undefined)).toEqual({
      value: null,
      fixed: false,
    });
  });

  it("fixed without rows", () => {
    expect(choice({ yes: 0, no: 0, na: 4 }, "yes")).toEqual({
      value: null,
      fixed: true,
    });
  });
});

describe("offered", () => {
  it("needs an answerable row", () => {
    expect(offered({ yes: 0, no: 0, na: 4 })).toBe(false);
    expect(offered({ yes: 0, no: 1, na: 0 })).toBe(true);
  });
});

describe("withFilter", () => {
  it("set and clear", () => {
    expect(withFilter(warframes, {}, "owned", "yes")).toEqual({ owned: "yes" });
    expect(withFilter(warframes, { owned: "yes" }, "owned", null)).toEqual({});
  });

  it("drops a conflicting selection", () => {
    expect(withFilter(warframes, { prime: "yes" }, "subsumed", "no")).toEqual({
      subsumed: "no",
    });
    expect(withFilter(warframes, { vaulted: "yes" }, "prime", "no")).toEqual({
      prime: "no",
    });
  });

  it("keeps a compatible selection", () => {
    expect(withFilter(warframes, { prime: "no" }, "owned", "yes")).toEqual({
      prime: "no",
      owned: "yes",
    });
  });
});

describe("prune", () => {
  it("drops a selection no row answers", () => {
    const mastered = [byName("Excalibur"), byName("Mesa Prime")];
    expect(prune(mastered, { mastered: "no" })).toEqual({});
    expect(prune(mastered, { mastered: "yes" })).toEqual({});
  });

  it("drops filters outside the category", () => {
    expect(prune(primaries, { subsumed: "no", prime: "yes" })).toEqual({
      prime: "yes",
    });
    expect(prune(warframes, { incarnon: "yes", owned: "yes" })).toEqual({
      owned: "yes",
    });
  });

  it("keeps a sibling of an empty selection", () => {
    expect(prune(primaries, { primeResurgence: "yes", owned: "no" })).toEqual({
      owned: "no",
    });
  });

  it("returns the same object when nothing changes", () => {
    const filters = { owned: "no" } as const;
    expect(prune(warframes, filters)).toBe(filters);
  });
});
