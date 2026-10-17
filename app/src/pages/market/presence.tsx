import { useEffect } from "react";
import { OptionSelect } from "@/components/option-select";
import { Checkbox } from "@/components/ui/checkbox";
import { api, logError, reportError } from "@/lib/bridge";
import { usePresenceStore } from "@/stores/presence-store";
import type { Presence, UserStatus } from "@/types";

const OPTIONS: { value: UserStatus; label: string }[] = [
  { value: "invisible", label: "Offline" },
  { value: "online", label: "Online" },
  { value: "ingame", label: "In game" },
];

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

  const push = async (next: Presence) => {
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
      <OptionSelect
        label="warframe.market status"
        value={presence.live ?? presence.status}
        options={OPTIONS}
        anyLabel="Status untouched"
        onChange={(status) => push({ ...presence, status })}
        size="sm"
        disabled={presence.auto}
      />
    </div>
  );
}
