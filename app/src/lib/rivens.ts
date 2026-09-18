import { percent } from "@/lib/format";
import type { AttributeGrade } from "@/types";

export function rollQuality(grade: number) {
  if (grade >= 0.9) {
    return { label: "S", variant: "default" as const };
  }
  if (grade >= 0.75) {
    return { label: "A", variant: "accent" as const };
  }
  if (grade >= 0.6) {
    return { label: "B", variant: "secondary" as const };
  }
  if (grade >= 0.4) {
    return { label: "C", variant: "muted" as const };
  }
  return { label: "D", variant: "warning" as const };
}

export function gradeTone(grade: string): string {
  if (grade === "S" || grade.startsWith("A")) {
    return "text-primary";
  }
  if (grade === "F") {
    return "text-warning";
  }
  return "text-muted-foreground";
}

export function attributeAmount(unit: string | null, value: number): string {
  if (unit === "multiply") {
    return `x${value.toFixed(2)}`;
  }
  const suffix = unit === "percent" ? "%" : unit === "seconds" ? "s" : "";
  const sign = value > 0 ? "+" : "";
  return `${sign}${value.toFixed(1)}${suffix}`;
}

export function attributeValue(attribute: AttributeGrade): string | null {
  return attribute.display === null
    ? null
    : attributeAmount(attribute.unit, attribute.display);
}

export function attributeRange(attribute: AttributeGrade): string | null {
  if (attribute.min === null || attribute.max === null) {
    return null;
  }
  const low = Math.min(attribute.min, attribute.max);
  const high = Math.max(attribute.min, attribute.max);
  return `${low.toFixed(1)} to ${high.toFixed(1)}`;
}

export function attributeText(attribute: AttributeGrade): string {
  const name = attribute.name ?? attribute.tag;
  const value = attributeValue(attribute);
  return value === null
    ? `${name} ${percent(attribute.percentile, 0)}`
    : `${value} ${name}`;
}

export function rivenAuctionUrl(weaponSlug: string): string {
  const query = new URLSearchParams({
    type: "riven",
    weapon_url_name: weaponSlug,
    sort_by: "price_asc",
  });
  return `https://warframe.market/auctions/search?${query.toString()}`;
}
