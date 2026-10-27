import { useEffect } from "react";
import { OptionSelect } from "@/components/option-select";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import { api, logError, reportError } from "@/lib/bridge";
import { usePresenceStore } from "@/stores/presence-store";
import type { MarketPresenceChoice, UserStatus } from "@/types";

const STATUS_LABELS: Record<UserStatus, string> = {
  offline: "Offline",
  invisible: "Offline",
  online: "Online",
  ingame: "In game",
};

const OPTIONS = (["invisible", "online", "ingame"] as UserStatus[]).map(
  (value) => ({ value, label: STATUS_LABELS[value] }),
);

export function LiveStatusBadge() {
  const live = usePresenceStore((state) => state.presence.live);
  if (live === null) {
    return <Badge variant="muted">Status unknown</Badge>;
  }

  const online = live === "online" || live === "ingame";
  return (
    <Badge variant={online ? "accent" : "muted"}>{STATUS_LABELS[live]}</Badge>
  );
}

export function PresenceControl() {
  const presence = usePresenceStore((state) => state.presence);
  const setPresence = usePresenceStore((state) => state.setPresence);

  useEffect(() => {
    async function load() {
      try {
        setPresence(await api.marketPresence());
      } catch (error) {
        logError("Reading the market presence", error);
      }
    }
    load();
  }, [setPresence]);

  const choose = async (choice: MarketPresenceChoice) => {
    try {
      await api.marketSetPresence(choice);
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
            choose({ status: presence.status, auto: checked === true })
          }
        />
        Follow the game
      </label>
      <OptionSelect
        label="warframe.market status"
        value={presence.status}
        options={OPTIONS}
        anyLabel="Status untouched"
        onChange={(status) => choose({ status, auto: presence.auto })}
        size="sm"
        disabled={presence.auto}
      />
    </div>
  );
}
