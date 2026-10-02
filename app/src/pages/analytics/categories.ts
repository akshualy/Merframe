import type { Slice } from "@/components/pie-chart";
import type { TradeCategory } from "@/types";

export const CATEGORY_LABELS: Record<TradeCategory, string> = {
  set: "Sets",
  prime: "Prime parts",
  riven: "Rivens",
  arcane: "Arcanes",
  relic: "Relics",
  mod: "Mods",
  other: "Other",
};

const CATEGORY_COLORS: Record<TradeCategory, string> = {
  set: "var(--chart-1)",
  prime: "var(--chart-2)",
  riven: "var(--chart-3)",
  arcane: "var(--chart-4)",
  relic: "var(--chart-5)",
  mod: "var(--chart-6)",
  other: "var(--muted-foreground)",
};

export const TRADE_CATEGORIES = Object.keys(CATEGORY_LABELS) as TradeCategory[];

export function categorySlices(
  value: (category: TradeCategory) => number,
): Slice[] {
  return TRADE_CATEGORIES.map((category) => ({
    key: category,
    label: CATEGORY_LABELS[category],
    color: CATEGORY_COLORS[category],
    value: value(category),
  }));
}
