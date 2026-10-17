import type { ReactNode } from "react";
import { GameIcon } from "@/components/game-icon";
import { cn } from "@/lib/utils";

type Currency = "platinum" | "ducats" | "credits";

const CURRENCIES: Record<Currency, { label: string; tone?: string }> = {
  platinum: { label: "Platinum", tone: "text-primary" },
  ducats: { label: "Ducats", tone: "text-accent" },
  credits: { label: "Credits" },
};

export function Amount({
  currency,
  size = 16,
  plain = false,
  className,
  children,
}: {
  currency: Currency;
  size?: number;
  plain?: boolean;
  className?: string;
  children: ReactNode;
}) {
  const { label, tone } = CURRENCIES[currency];
  return (
    <span
      className={cn(
        "flex items-center gap-0.5 tabular-nums",
        !plain && tone,
        className,
      )}
    >
      {children}
      <GameIcon name={currency} size={size} alt={label} />
    </span>
  );
}
