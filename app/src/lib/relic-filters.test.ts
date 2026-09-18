import { describe, expect, it } from "vitest";
import {
  ANY_REFINEMENT,
  highestOwnedRefinement,
  isFavourite,
  matchesRelicFilters,
  partIdentity,
  type RelicFilters,
  refinementValue,
  wantedChance,
  wantedKeysOf,
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

describe("partIdentity", () => {
  it("strips the blueprint suffix", () => {
    expect(
      partIdentity("/Lotus/Types/Recipes/Weapons/AlternoxPrimeBlueprint"),
    ).toBe("/Lotus/Types/Recipes/Weapons/AlternoxPrime");
  });

  it("strips the component suffix", () => {
    expect(
      partIdentity(
        "/Lotus/Types/Recipes/WarframeRecipes/LavosPrimeSystemsComponent",
      ),
    ).toBe("/Lotus/Types/Recipes/WarframeRecipes/LavosPrimeSystems");
  });

  it("leaves other parts alone", () => {
    const receiver =
      "/Lotus/Types/Recipes/Weapons/WeaponParts/KompressaPrimeReceiver";
    expect(partIdentity(receiver)).toBe(receiver);
  });
});

describe("wantedKeysOf", () => {
  it("one key per part identity", () => {
    expect(
      wantedKeysOf([
        "/Lotus/Types/Recipes/Weapons/AlternoxPrimeBlueprint",
        "/Lotus/Types/Recipes/Weapons/AlternoxPrimeComponent",
      ]),
    ).toEqual(new Set(["/Lotus/Types/Recipes/Weapons/AlternoxPrime"]));
  });
});

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

describe("wantedChance", () => {
  const axi = relic("Axi A20");
  const intact = refinementValue(axi, "Intact");
  const alternox = wantedKeysOf([
    "/Lotus/Types/Recipes/Weapons/AlternoxPrimeBlueprint",
  ]);

  it("single drop chance for one player", () => {
    if (!intact) {
      throw new Error("Intact value missing");
    }
    expect(wantedChance(axi, intact, alternox, 1)).toBeCloseTo(2);
  });

  it("compounds over the squad", () => {
    if (!intact) {
      throw new Error("Intact value missing");
    }
    expect(wantedChance(axi, intact, alternox, 4)).toBeCloseTo(
      (1 - 0.98 ** 4) * 100,
    );
  });

  it("zero when nothing wanted drops", () => {
    if (!intact) {
      throw new Error("Intact value missing");
    }
    expect(wantedChance(axi, intact, new Set(["/Lotus/Nothing"]), 4)).toBe(0);
  });
});

describe("isFavourite", () => {
  it("favourite relic or favourite reward", () => {
    expect(isFavourite(relic("Axi A20"))).toBe(true);
    expect(isFavourite(relic("Lith B11"))).toBe(false);
  });
});

describe("matchesRelicFilters", () => {
  const axi = relic("Axi A20");
  const none = new Set<string>();

  it("open filters match", () => {
    expect(matchesRelicFilters(axi, open, none)).toBe(true);
  });

  it("wanted parts must drop", () => {
    const alternox = wantedKeysOf([
      "/Lotus/Types/Recipes/Weapons/AlternoxPrimeBlueprint",
    ]);
    expect(matchesRelicFilters(axi, open, alternox)).toBe(true);
    expect(matchesRelicFilters(relic("Lith B11"), open, alternox)).toBe(false);
  });

  it("favourite", () => {
    expect(matchesRelicFilters(axi, { ...open, favourite: "yes" }, none)).toBe(
      true,
    );
    expect(matchesRelicFilters(axi, { ...open, favourite: "no" }, none)).toBe(
      false,
    );
  });

  it("vaulted", () => {
    expect(matchesRelicFilters(axi, { ...open, vaulted: "no" }, none)).toBe(
      false,
    );
  });

  it("stacked means ten or more", () => {
    expect(matchesRelicFilters(axi, { ...open, stacked: "yes" }, none)).toBe(
      false,
    );
    expect(
      matchesRelicFilters(relic("Lith P5"), { ...open, stacked: "yes" }, none),
    ).toBe(true);
  });

  it("refined excludes intact only stacks", () => {
    expect(matchesRelicFilters(axi, { ...open, refined: "no" }, none)).toBe(
      true,
    );
  });

  it("tier owned", () => {
    expect(
      matchesRelicFilters(axi, { ...open, tierOwned: "Intact" }, none),
    ).toBe(true);
    expect(
      matchesRelicFilters(axi, { ...open, tierOwned: "Radiant" }, none),
    ).toBe(false);
  });
});
