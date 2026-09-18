import { useEffect, useState } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useListen } from "@/hooks/use-listen";
import { api, events, logError, reportError } from "@/lib/bridge";
import type { MarketPresence, MarketStatus } from "@/types";

const UNSET = "unset";

const OPTIONS: { value: MarketStatus; label: string }[] = [
  { value: "invisible", label: "Offline" },
  { value: "online", label: "Online" },
  { value: "ingame", label: "In game" },
];

export function PresenceControl() {
  const [presence, setPresence] = useState<MarketPresence>({
    status: null,
    auto: false,
  });

  useEffect(() => {
    async function load() {
      try {
        setPresence(await api.marketPresence());
      } catch (error) {
        logError("Reading the market presence", error);
      }
    }
    load();
  }, []);

  useListen<MarketStatus>(events.marketPresence, (status) =>
    setPresence((current) => ({ ...current, status })),
  );

  const push = async (next: MarketPresence) => {
    setPresence(next);
    try {
      await api.marketSetPresence(next.status, next.auto);
    } catch (error) {
      reportError(error);
    }
  };

  return (
    <div className="flex items-center gap-2">
      <label className="text-muted-foreground flex cursor-pointer items-center gap-2 text-xs">
        <Checkbox
          checked={presence.auto}
          onCheckedChange={(checked) =>
            push({ ...presence, auto: checked === true })
          }
        />
        Follow the game
      </label>
      <Select
        value={presence.status ?? UNSET}
        disabled={presence.auto}
        onValueChange={(value) =>
          push({
            ...presence,
            status: value === UNSET ? null : (value as MarketStatus),
          })
        }
      >
        <SelectTrigger size="sm" aria-label="warframe.market status">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={UNSET}>Status untouched</SelectItem>
          {OPTIONS.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
