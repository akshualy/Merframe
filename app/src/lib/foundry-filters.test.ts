import { describe, expect, it } from "vitest";
import { filtersFor, foundryMatches, scoped } from "@/lib/foundry-filters";
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

describe("filtersFor", () => {
  it("helminth filters only for warframes", () => {
    const keys = (category: "warframe" | "primary") =>
      filtersFor(category).map((group) => group.key);
    expect(keys("warframe")).toContain("subsumed");
    expect(keys("primary")).not.toContain("subsumed");
    expect(keys("primary")).toContain("prime");
  });
});

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

  it("vaulted follows the prime vault state", () => {
    const vaulted = items.find((item) => item.prime?.vault === "vaulted");
    expect(vaulted).toBeDefined();
    if (vaulted) {
      expect(foundryMatches(vaulted, "", { vaulted: "yes" })).toBe(true);
    }
  });
});

describe("scoped", () => {
  it("drops filters the category does not show", () => {
    expect(scoped({ prime: "yes", subsumed: "no" }, "primary")).toEqual({
      prime: "yes",
    });
  });

  it("keeps warframe filters for warframes", () => {
    expect(scoped({ subsumed: "no" }, "warframe")).toEqual({ subsumed: "no" });
  });
});
