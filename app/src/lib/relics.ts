export const RELIC_TIERS = [
  "Lith",
  "Meso",
  "Neo",
  "Axi",
  "Requiem",
  "Omnia",
] as const;

export type RelicTier = (typeof RELIC_TIERS)[number];

export function isRelicTier(tier: string): tier is RelicTier {
  return (RELIC_TIERS as readonly string[]).includes(tier);
}

export const REFINEMENTS = [
  "Intact",
  "Exceptional",
  "Flawless",
  "Radiant",
] as const;

export function refinementTone(refinement: string): string {
  switch (refinement) {
    case "Radiant":
      return "text-primary";
    case "Flawless":
      return "text-accent";
    case "Exceptional":
      return "text-foreground";
    default:
      return "text-muted-foreground";
  }
}
