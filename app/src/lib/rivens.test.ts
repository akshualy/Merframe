import { describe, expect, it } from "vitest";
import {
  attributeAmount,
  attributeRange,
  attributeText,
  attributeValue,
  gradeTone,
  rivenAuctionUrl,
  rollQuality,
} from "@/lib/rivens";
import type { AttributeGrade } from "@/types";

const initialCombo: AttributeGrade = {
  tag: "WeaponMeleeComboInitialBonusMod",
  name: "Initial combo",
  slug: null,
  unit: null,
  prefix: null,
  suffix: null,
  localization: null,
  value: 2.2,
  percentile: 0.5,
  grade: "B",
  multiplier: 1,
  rolled: 2.2,
  display: 2.2,
  min: 1,
  max: 3,
  curse: false,
};

const attackSpeed: AttributeGrade = {
  tag: "WeaponFireRateMod",
  name: "Fire Rate / Attack Speed",
  slug: null,
  unit: "percent",
  prefix: null,
  suffix: null,
  localization: null,
  value: 5,
  percentile: 0.91,
  grade: "A",
  multiplier: 1,
  rolled: 5,
  display: 5,
  min: 4.1,
  max: 5,
  curse: false,
};

describe("rollQuality", () => {
  it("letter per grade band", () => {
    expect(rollQuality(0.95).label).toBe("S");
    expect(rollQuality(0.8423).label).toBe("A");
    expect(rollQuality(0.6).label).toBe("B");
    expect(rollQuality(0.5).label).toBe("C");
    expect(rollQuality(0.1).label).toBe("D");
  });

  it("warning tone for the lowest band", () => {
    expect(rollQuality(0.1).variant).toBe("warning");
  });
});

describe("gradeTone", () => {
  it("primary for S and A grades", () => {
    expect(gradeTone("S")).toBe("text-primary");
    expect(gradeTone("A-")).toBe("text-primary");
  });

  it("warning for F, muted otherwise", () => {
    expect(gradeTone("F")).toBe("text-warning");
    expect(gradeTone("B+")).toBe("text-muted-foreground");
  });
});

describe("attributeAmount", () => {
  it("percent with sign", () => {
    expect(attributeAmount("percent", 96.4)).toBe("+96.4%");
    expect(attributeAmount("percent", -42.3)).toBe("-42.3%");
  });

  it("seconds", () => {
    expect(attributeAmount("seconds", 1.2)).toBe("+1.2s");
  });

  it("multiplier", () => {
    expect(attributeAmount("multiply", 2.222)).toBe("x2.22");
  });

  it("plain number", () => {
    expect(attributeAmount(null, 2.2)).toBe("+2.2");
  });
});

describe("attributeValue", () => {
  it("formats the display value", () => {
    expect(attributeValue(attackSpeed)).toBe("+5.0%");
  });

  it("null without a display value", () => {
    expect(attributeValue({ ...attackSpeed, display: null })).toBeNull();
  });
});

describe("attributeRange", () => {
  it("low to high", () => {
    expect(attributeRange(attackSpeed)).toBe("4.1 to 5.0");
  });

  it("orders a negative range", () => {
    expect(attributeRange({ ...attackSpeed, min: -1.5, max: -3 })).toBe(
      "-3.0 to -1.5",
    );
  });

  it("null without bounds", () => {
    expect(attributeRange({ ...attackSpeed, min: null })).toBeNull();
  });
});

describe("attributeText", () => {
  it("value then name", () => {
    expect(attributeText(initialCombo)).toBe("+2.2 Initial combo");
  });

  it("name then percentile without a value", () => {
    expect(attributeText({ ...attackSpeed, display: null })).toBe(
      "Fire Rate / Attack Speed 91%",
    );
  });

  it("tag when the name is unknown", () => {
    expect(attributeText({ ...initialCombo, name: null })).toBe(
      "+2.2 WeaponMeleeComboInitialBonusMod",
    );
  });
});

describe("rivenAuctionUrl", () => {
  it("cheapest auctions for the weapon", () => {
    expect(rivenAuctionUrl("kuva_shildeg")).toBe(
      "https://warframe.market/auctions/search?type=riven&weapon_url_name=kuva_shildeg&sort_by=price_asc",
    );
  });
});
