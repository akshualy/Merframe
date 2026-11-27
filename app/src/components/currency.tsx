import type { ReactNode } from "react";
import { GameIcon } from "@/components/game-icon";
import { num } from "@/lib/format";
import { cn } from "@/lib/utils";

type Currency = "platinum" | "ducats" | "credits";

const CURRENCIES: Record<Currency, { label: string; tone?: string }> = {
  platinum: { label: "Platinum", tone: "text-primary" },
  ducats: { label: "Ducats", tone: "text-accent" },
  credits: { label: "Credits" },
};

export function Amount({
  currency,
  plain = false,
  className,
  children,
}: {
  currency: Currency;
  plain?: boolean;
  className?: string;
  children: ReactNode;
}) {
  const { label, tone } = CURRENCIES[currency];
  return (
    <span className={cn("flex items-center gap-1", !plain && tone, className)}>
      {children}
      <GameIcon name={currency} size="1.15em" alt={label} />
    </span>
  );
}

export function PlatinumAndDucats({
  platinum,
  ducats,
}: {
  platinum: number;
  ducats: number | null;
}) {
  return (
    <span className="flex items-center gap-3">
      <Amount currency="platinum">{num(platinum)}</Amount>
      {ducats !== null && <Amount currency="ducats">{num(ducats)}</Amount>}
    </span>
  );
}
