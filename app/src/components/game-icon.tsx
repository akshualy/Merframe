import { Badge } from "@/components/ui/badge";
import { isRelicTier } from "@/lib/relics";
import { cn } from "@/lib/utils";
import { capitalize } from "@/lib/world";

const MONOCHROME_GLYPHS = [
  "day",
  "night",
  "warm",
  "cold",
  "baro",
  "sortie",
  "arbitration",
  "fissure",
  "invasion",
  "nightwave",
  "duviri",
  "helminth",
  "riven",
  "archon-hunt",
  "darvo",
  "varzia",
  "vaulted",
  "archon-shard",
  "relic",
  "relic-lith",
  "relic-meso",
  "relic-neo",
  "relic-axi",
  "relic-requiem",
  "mastered",
  "polarity-madurai",
  "polarity-vazarin",
  "polarity-naramon",
  "polarity-zenurik",
  "polarity-unairu",
  "polarity-penjaga",
  "polarity-umbra",
  "polarity-any",
] as const;

const COLOURED_GLYPHS = [
  "fass",
  "vome",
  "corpus",
  "grineer",
  "duviri-joy",
  "duviri-anger",
  "duviri-envy",
  "duviri-sorrow",
  "duviri-fear",
  "forma",
  "orokin-reactor",
  "orokin-catalyst",
  "exilus-adapter",
  "exilus-weapon-adapter",
  "arcane-common",
  "arcane-uncommon",
  "arcane-rare",
  "arcane-legendary",
  "platinum",
  "credits",
  "ducats",
] as const;

export type GameIconName =
  | (typeof MONOCHROME_GLYPHS)[number]
  | (typeof COLOURED_GLYPHS)[number];

const MONOCHROME: ReadonlySet<GameIconName> = new Set(MONOCHROME_GLYPHS);

export type ArcaneRarity = "Common" | "Uncommon" | "Rare" | "Legendary";

export const ARCANE_BACKDROP: Record<ArcaneRarity, GameIconName> = {
  Common: "arcane-common",
  Uncommon: "arcane-uncommon",
  Rare: "arcane-rare",
  Legendary: "arcane-legendary",
};

const CYCLE_STATES: Record<string, GameIconName> = {
  day: "day",
  night: "night",
  fass: "fass",
  vome: "vome",
  warm: "warm",
  cold: "cold",
  corpus: "corpus",
  grineer: "grineer",
  joy: "duviri-joy",
  anger: "duviri-anger",
  envy: "duviri-envy",
  sorrow: "duviri-sorrow",
  fear: "duviri-fear",
};

const TIER_ICONS: Record<string, GameIconName> = {
  Lith: "relic-lith",
  Meso: "relic-meso",
  Neo: "relic-neo",
  Axi: "relic-axi",
  Requiem: "relic-requiem",
};

const POLARITY_SCHOOLS: Record<string, string> = {
  AP_ATTACK: "madurai",
  AP_DEFENSE: "vazarin",
  AP_TACTIC: "naramon",
  AP_POWER: "zenurik",
  AP_WARD: "unairu",
  AP_PRECEPT: "penjaga",
  AP_UMBRA: "umbra",
  AP_ANY: "any",
  AP_UNIVERSAL: "any",
};

const POLARITY_ICONS: Record<string, GameIconName> = {
  madurai: "polarity-madurai",
  vazarin: "polarity-vazarin",
  naramon: "polarity-naramon",
  zenurik: "polarity-zenurik",
  unairu: "polarity-unairu",
  penjaga: "polarity-penjaga",
  umbra: "polarity-umbra",
  any: "polarity-any",
};

export function cycleIcon(state: string): GameIconName | null {
  return CYCLE_STATES[state] ?? null;
}

export function relicTierIcon(tier: string): GameIconName {
  return TIER_ICONS[tier] ?? "relic";
}

export function GameIcon({
  name,
  size = 20,
  className,
  alt = "",
  title,
}: {
  name: GameIconName;
  size?: number;
  className?: string;
  alt?: string;
  title?: string;
}) {
  if (MONOCHROME.has(name)) {
    return (
      <svg
        role={alt ? "img" : undefined}
        aria-label={alt || undefined}
        aria-hidden={alt === "" || undefined}
        viewBox="0 0 128 128"
        width={size}
        height={size}
        fill="currentColor"
        className={cn("shrink-0", className)}
      >
        {title && <title>{title}</title>}
        <use href={`/game/${name}.svg#glyph`} />
      </svg>
    );
  }

  return (
    <img
      src={`/game/${name}.png`}
      alt={alt}
      title={title}
      width={size}
      height={size}
      draggable={false}
      className={cn("shrink-0 object-contain select-none", className)}
      style={{ width: size, height: size }}
    />
  );
}

export function LucideSlotGameIcon({
  name,
  className,
}: {
  name: GameIconName;
  className?: string;
}) {
  return <GameIcon name={name} size={20} className={cn("-m-0.5", className)} />;
}

export function RelicTierIcon({ tier, size }: { tier: string; size: number }) {
  if (!isRelicTier(tier)) {
    return <Badge variant="muted">{tier}</Badge>;
  }
  return <GameIcon name={relicTierIcon(tier)} size={size} alt="" />;
}

export function PolarityIcon({
  polarity,
  size = 14,
  className,
}: {
  polarity: string;
  size?: number;
  className?: string;
}) {
  const school = POLARITY_SCHOOLS[polarity] ?? polarity.toLowerCase();
  const name = POLARITY_ICONS[school];
  if (!name) {
    return null;
  }
  const label = capitalize(school);
  return (
    <GameIcon
      name={name}
      size={size}
      className={className}
      alt={label}
      title={label}
    />
  );
}
