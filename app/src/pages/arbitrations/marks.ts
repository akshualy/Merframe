import { useCallback, useState } from "react";
import { toggled } from "@/lib/filters";
import { dateTime } from "@/lib/format";
import { notify } from "@/lib/toast";
import { useAppStore, useSettings } from "@/stores/app-store";
import type { ArbitrationAlertSettings, ArbitrationSlot } from "@/types";

export function useArbitrationMarks() {
  const settings = useSettings();
  const saveSettings = useAppStore((state) => state.saveSettings);
  const [busy, setBusy] = useState(false);

  const patch = useCallback(
    async (next: Partial<ArbitrationAlertSettings>) => {
      setBusy(true);
      try {
        return await saveSettings({
          alerts: {
            ...settings.alerts,
            arbitrations: { ...settings.alerts.arbitrations, ...next },
          },
        });
      } finally {
        setBusy(false);
      }
    },
    [saveSettings, settings.alerts],
  );

  const toggleFavourite = useCallback(
    (nodeId: string) =>
      patch({
        favourite_nodes: toggled(
          settings.alerts.arbitrations.favourite_nodes,
          nodeId,
        ),
      }),
    [patch, settings.alerts.arbitrations.favourite_nodes],
  );

  const toggleReminder = useCallback(
    async (slot: ArbitrationSlot) => {
      const error = await patch({
        reminders: toggled(settings.alerts.arbitrations.reminders, slot.id),
      });
      if (error) {
        return;
      }

      if (slot.reminder) {
        notify.success("Reminder removed");
      } else {
        notify.success("Reminder saved", {
          description: `${slot.node}, ${dateTime(slot.starts)}`,
        });
      }
    },
    [patch, settings.alerts.arbitrations.reminders],
  );

  return { busy, toggleFavourite, toggleReminder };
}
