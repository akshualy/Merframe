import { percent } from "@/lib/format";
import type { AttributeGrade } from "@/types";

export function gradeVariant(grade: string) {
  if (grade === "S") {
    return "default";
  }
  if (grade === "A") {
    return "accent";
  }
  if (grade === "B") {
    return "secondary";
  }
  if (grade === "C") {
    return "muted";
  }
  return "warning";
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

export function attributeLabel(attribute: AttributeGrade): string {
  return attribute.name ?? attribute.tag;
}

export function attributeText(attribute: AttributeGrade): string {
  const name = attributeLabel(attribute);
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
