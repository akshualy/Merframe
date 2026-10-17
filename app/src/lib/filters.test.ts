import { describe, expect, it } from "vitest";
import {
  passesYesNo,
  plainOptions,
  withYesNo,
  type YesNo,
  yesNoOptions,
} from "@/lib/filters";

describe("passesYesNo", () => {
  const flags = { prime: true, vaulted: false };
  const has = (key: keyof typeof flags) => flags[key];

  it("passes with no filters", () => {
    expect(passesYesNo({}, has)).toBe(true);
  });

  it("yes requires the flag", () => {
    expect(passesYesNo({ prime: "yes" }, has)).toBe(true);
    expect(passesYesNo({ vaulted: "yes" }, has)).toBe(false);
  });

  it("no requires the flag off", () => {
    expect(passesYesNo({ vaulted: "no" }, has)).toBe(true);
    expect(passesYesNo({ prime: "no" }, has)).toBe(false);
  });

  it("every filter must pass", () => {
    expect(passesYesNo({ prime: "yes", vaulted: "yes" }, has)).toBe(false);
  });
});

describe("withYesNo", () => {
  const filters: Partial<Record<"prime" | "vaulted", YesNo>> = {
    prime: "yes",
    vaulted: "no",
  };

  it("sets a mode and drops a cleared one", () => {
    expect(withYesNo({}, "prime", "yes")).toEqual({ prime: "yes" });
    expect(withYesNo(filters, "prime", null)).toEqual({
      vaulted: "no",
    });
  });

  it("leaves the given filters untouched", () => {
    withYesNo(filters, "prime", "no");
    expect(filters).toEqual({ prime: "yes", vaulted: "no" });
  });
});

describe("yesNoOptions", () => {
  it("labels the two choices", () => {
    expect(yesNoOptions("Prime", "Non-prime")).toEqual([
      { value: "yes", label: "Prime" },
      { value: "no", label: "Non-prime" },
    ]);
  });
});

describe("plainOptions", () => {
  it("labels each value with itself", () => {
    expect(plainOptions(["Lith", "Axi"])).toEqual([
      { value: "Lith", label: "Lith" },
      { value: "Axi", label: "Axi" },
    ]);
  });
});
