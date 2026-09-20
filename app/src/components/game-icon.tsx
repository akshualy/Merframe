import { Badge } from "@/components/ui/badge";
import { isRelicTier } from "@/lib/relics";
import { cn } from "@/lib/utils";

export type GameIconName =
  | "day"
  | "night"
  | "fass"
  | "vome"
  | "warm"
  | "cold"
  | "corpus"
  | "grineer"
  | "duviri-joy"
  | "duviri-anger"
  | "duviri-envy"
  | "duviri-sorrow"
  | "duviri-fear"
  | "baro"
  | "sortie"
  | "archon"
  | "archon-amar"
  | "archon-boreal"
  | "archon-nira"
  | "archon-shard"
  | "forma"
  | "orokin-reactor"
  | "orokin-catalyst"
  | "exilus-adapter"
  | "exilus-weapon-adapter"
  | "relic"
  | "relic-lith"
  | "relic-meso"
  | "relic-neo"
  | "relic-axi"
  | "relic-requiem"
  | "arcane-common"
  | "arcane-uncommon"
  | "arcane-rare"
  | "arcane-legendary"
  | "mastered"
  | "polarity-madurai"
  | "polarity-vazarin"
  | "polarity-naramon"
  | "polarity-zenurik"
  | "polarity-unairu"
  | "polarity-penjaga"
  | "polarity-umbra"
  | "polarity-any"
  | "platinum"
  | "credits"
  | "ducats";

export type ArcaneRarity = "Common" | "Uncommon" | "Rare" | "Legendary";

export const ARCANE_BACKDROP: Record<ArcaneRarity, GameIconName> = {
  Common: "arcane-common",
  Uncommon: "arcane-uncommon",
  Rare: "arcane-rare",
  Legendary: "arcane-legendary",
};

const MONOCHROME: ReadonlySet<GameIconName> = new Set([
  "day",
  "night",
  "warm",
  "cold",
  "baro",
  "sortie",
  "archon",
  "archon-amar",
  "archon-boreal",
  "archon-nira",
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
]);

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

const ARCHONS: Record<string, GameIconName> = {
  AMAR: "archon-amar",
  BOREAL: "archon-boreal",
  NIRA: "archon-nira",
};

export function cycleIcon(state: string): GameIconName | null {
  return CYCLE_STATES[state] ?? null;
}

export function relicTierIcon(tier: string): GameIconName {
  return TIER_ICONS[tier] ?? "relic";
}

export function archonIcon(boss: string): GameIconName {
  const name = boss.replace(/^SORTIE_BOSS_/, "").replace(/^ARCHON_/, "");
  return ARCHONS[name] ?? "archon";
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
  return (
    <img
      src={`/game/${name}.png`}
      alt={alt}
      title={title}
      width={size}
      height={size}
      draggable={false}
      className={cn(
        "shrink-0 object-contain select-none",
        MONOCHROME.has(name) && "invert dark:invert-0",
        className,
      )}
      style={{ width: size, height: size }}
    />
  );
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
  const label = school.charAt(0).toUpperCase() + school.slice(1);
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
