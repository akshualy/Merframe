import { Receipt, ShoppingCart } from "lucide-react";
import { Amount } from "@/components/currency";
import { Button } from "@/components/ui/button";
import { Dash } from "@/components/ui/dash";
import { Hint } from "@/components/ui/hint";
import { num } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { OrderType } from "@/types";

export function RankedPlat({
  side,
  label,
  value,
  floor,
  tone,
  className,
  onOpen,
}: {
  side: OrderType;
  label?: string;
  value: number | null;
  floor?: boolean;
  tone?: string;
  className?: string;
  onOpen: () => void;
}) {
  return (
    <Button
      variant="outline"
      size="sm"
      className={cn(
        "h-8 max-w-36 basis-20 grow justify-between gap-1.5 px-2.5",
        className,
      )}
      onClick={onOpen}
      title={`${side === "buy" ? "Buy" : "Sell"} on warframe.market`}
    >
      <Hint as="span">
        {label ??
          (side === "sell" ? (
            <Receipt className="size-4" />
          ) : (
            <ShoppingCart className="size-4" />
          ))}
      </Hint>
      {value === null ? (
        <Dash />
      ) : (
        <Amount currency="platinum" className={cn("text-sm font-bold", tone)}>
          {num(value)}
          {floor ? "+" : ""}
        </Amount>
      )}
    </Button>
  );
}
