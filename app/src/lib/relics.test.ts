import { describe, expect, it } from "vitest";
import { isRelicTier } from "@/lib/relics";

describe("isRelicTier", () => {
  it("knows the six tiers", () => {
    expect(isRelicTier("Lith")).toBe(true);
    expect(isRelicTier("Omnia")).toBe(true);
  });

  it("rejects a raw modifier", () => {
    expect(isRelicTier("VoidT7")).toBe(false);
  });
});
