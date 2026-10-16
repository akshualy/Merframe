import { describe, expect, it } from "vitest";
import {
  ANY_REFINEMENT,
  highestOwnedRefinement,
  matchesRelicFilters,
  type RelicFilters,
  refinementValue,
  showsWanted,
} from "@/lib/relic-filters";
import type { RelicPlan } from "@/types";
import planner from "./relic-filters.test-data.json";

const plans = planner.plans as RelicPlan[];
const relic = (name: string) => {
  const plan = plans.find((entry) => entry.relic === name);
  if (!plan) {
    throw new Error(`${name} missing from the test data`);
  }
  return plan;
};

const open: RelicFilters = {
  squadSize: 4,
  refinement: ANY_REFINEMENT,
  favourite: null,
  vaulted: null,
  setsOwned: null,
  itemsOwned: null,
  stacked: null,
  refined: null,
  tierOwned: null,
  wanted: [],
};

describe("refinementValue", () => {
  const axi = relic("Axi A20");

  it("best refinement for any", () => {
    expect(refinementValue(axi, ANY_REFINEMENT)?.refinement).toBe(
      axi.best.refinement,
    );
  });

  it("named refinement", () => {
    expect(refinementValue(axi, "Radiant")?.refinement).toBe("Radiant");
  });
});

describe("highestOwnedRefinement", () => {
  const axi = relic("Axi A20");

  it("intact only", () => {
    expect(highestOwnedRefinement(axi)).toBe("Intact");
  });

  it("highest with copies", () => {
    const plan: RelicPlan = {
      ...axi,
      ownership: {
        ...axi.ownership,
        by_refinement: [
          { refinement: "Intact", count: 2 },
          { refinement: "Radiant", count: 1 },
          { refinement: "Flawless", count: 0 },
        ],
      },
    };
    expect(highestOwnedRefinement(plan)).toBe("Radiant");
  });
});

describe("showsWanted", () => {
  it("only when a plan carries a wanted reward", () => {
    expect(showsWanted(plans)).toBe(true);
    expect(showsWanted([relic("Lith B11"), relic("Lith P5")])).toBe(false);
  });
});

describe("matchesRelicFilters", () => {
  const axi = relic("Axi A20");

  it("open filters match", () => {
    expect(matchesRelicFilters(axi, open)).toBe(true);
  });

  it("favourite", () => {
    expect(matchesRelicFilters(axi, { ...open, favourite: "yes" })).toBe(true);
    expect(matchesRelicFilters(axi, { ...open, favourite: "no" })).toBe(false);
  });

  it("vaulted", () => {
    expect(matchesRelicFilters(axi, { ...open, vaulted: "no" })).toBe(false);
  });

  it("stacked means ten or more", () => {
    expect(matchesRelicFilters(axi, { ...open, stacked: "yes" })).toBe(false);
    expect(
      matchesRelicFilters(relic("Lith P5"), { ...open, stacked: "yes" }),
    ).toBe(true);
  });

  it("refined excludes intact only stacks", () => {
    expect(matchesRelicFilters(axi, { ...open, refined: "no" })).toBe(true);
  });

  it("tier owned", () => {
    expect(matchesRelicFilters(axi, { ...open, tierOwned: "Intact" })).toBe(
      true,
    );
    expect(matchesRelicFilters(axi, { ...open, tierOwned: "Radiant" })).toBe(
      false,
    );
  });
});
