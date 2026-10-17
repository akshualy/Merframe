import { describe, expect, it } from "vitest";
import { changeColor, signed } from "@/lib/chart";
import { num } from "@/lib/format";

describe("signed", () => {
  it("plus sign only on gains", () => {
    expect(signed(1200, num)).toBe("+1,200");
    expect(signed(-3, num)).toBe("-3");
    expect(signed(0, num)).toBe("0");
  });
});

describe("changeColor", () => {
  it("loss below zero, otherwise gain", () => {
    expect(changeColor(-1)).toBe("var(--chart-loss)");
    expect(changeColor(0)).toBe("var(--chart-gain)");
    expect(changeColor(2)).toBe("var(--chart-gain)");
  });
});
