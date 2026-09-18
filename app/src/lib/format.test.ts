import { describe, expect, it } from "vitest";
import {
  ago,
  countdown,
  dateTime,
  dayLabel,
  displayNameFromPath,
  marketUrl,
  monthLabel,
  num,
  percent,
  plat,
  secondsUntil,
  spansYears,
} from "@/lib/format";

describe("num", () => {
  it("rounds and groups thousands", () => {
    expect(num(7192988.4)).toBe("7,192,988");
  });

  it("dash for missing", () => {
    expect(num(null)).toBe("-");
    expect(num(undefined)).toBe("-");
  });
});

describe("plat", () => {
  it("one decimal", () => {
    expect(plat(108.5)).toBe("108.5");
    expect(plat(86)).toBe("86.0");
  });

  it("dash for missing", () => {
    expect(plat(null)).toBe("-");
  });
});

describe("percent", () => {
  it("percent of fraction", () => {
    expect(percent(0.9891)).toBe("98.9%");
  });

  it("digits", () => {
    expect(percent(0.9413944666884638, 0)).toBe("94%");
  });

  it("dash for missing", () => {
    expect(percent(undefined)).toBe("-");
  });
});

describe("countdown", () => {
  it("ready at zero", () => {
    expect(countdown(0)).toBe("ready");
    expect(countdown(-5)).toBe("ready");
  });

  it("days and hours", () => {
    expect(countdown(90000)).toBe("1d 1h");
  });

  it("hours and padded minutes", () => {
    expect(countdown(3720)).toBe("1h 02m");
  });

  it("minutes and padded seconds", () => {
    expect(countdown(125)).toBe("2m 05s");
  });

  it("seconds only", () => {
    expect(countdown(42)).toBe("42s");
  });
});

describe("secondsUntil", () => {
  it("seconds between now and the instant", () => {
    const now = Date.parse("2026-09-15T11:59:00Z");
    expect(secondsUntil("2026-09-15T12:00:00Z", now)).toBe(60);
  });

  it("negative once passed", () => {
    const now = Date.parse("2026-09-15T12:00:30Z");
    expect(secondsUntil("2026-09-15T12:00:00Z", now)).toBe(-30);
  });
});

describe("ago", () => {
  it("never without a date", () => {
    expect(ago(null)).toBe("never");
    expect(ago("")).toBe("never");
  });

  it("minutes ago", () => {
    expect(ago(new Date(Date.now() - 120_000).toISOString())).toBe("2m ago");
  });

  it("hours ago", () => {
    expect(ago(new Date(Date.now() - 3 * 3_600_000).toISOString())).toBe(
      "3h ago",
    );
  });

  it("clamps future dates to zero", () => {
    expect(ago(new Date(Date.now() + 60_000).toISOString())).toBe("0s ago");
  });
});

describe("dateTime", () => {
  it("carries the year", () => {
    expect(dateTime("2026-03-04T10:30:00Z")).toContain("2026");
  });
});

describe("spansYears", () => {
  it("true across new year", () => {
    expect(spansYears(["2025-12-31", "2026-01-01"])).toBe(true);
  });

  it("false within one year", () => {
    expect(spansYears(["2026-01-01", "2026-06-30"])).toBe(false);
  });
});

describe("dayLabel", () => {
  it("day and month", () => {
    expect(dayLabel("2026-03-04", false)).toBe("04 Mar");
  });

  it("with year", () => {
    expect(dayLabel("2026-03-04", true)).toBe("04 Mar 2026");
  });
});

describe("monthLabel", () => {
  it("month only", () => {
    expect(monthLabel("2026-03-04", false)).toBe("Mar");
  });

  it("with year", () => {
    expect(monthLabel("2026-03-04", true)).toBe("Mar 2026");
  });
});

describe("displayNameFromPath", () => {
  it("splits the camel case tail", () => {
    expect(
      displayNameFromPath(
        "/Lotus/Types/Recipes/Weapons/WeaponParts/KompressaPrimeReceiver",
      ),
    ).toBe("Kompressa Prime Receiver");
  });

  it("drops the Component suffix", () => {
    expect(
      displayNameFromPath(
        "/Lotus/Types/Recipes/WarframeRecipes/LavosPrimeSystemsComponent",
      ),
    ).toBe("Lavos Prime Systems");
  });

  it("bare name without slashes", () => {
    expect(displayNameFromPath("AlternoxPrimeBlueprint")).toBe(
      "Alternox Prime Blueprint",
    );
  });
});

describe("marketUrl", () => {
  it("item page for a slug", () => {
    expect(marketUrl("axi_a20_relic")).toBe(
      "https://warframe.market/items/axi_a20_relic",
    );
  });
});
