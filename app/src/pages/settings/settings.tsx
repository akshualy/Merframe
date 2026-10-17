import { Save, TriangleAlert } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Page } from "@/components/page";
import { Button } from "@/components/ui/button";
import { api, reportError } from "@/lib/bridge";
import { notify } from "@/lib/toast";
import { useAppStore, useSettings } from "@/stores/app-store";
import type { Settings } from "@/types";
import { PricesAndData, StatsTabSetting } from "./data";
import { FissureAlerts } from "./fissures";
import { NotificationChannels } from "./notifications";
import { CycleTimers } from "./timers";

export function SettingsPage() {
  const stored = useSettings();
  const setStored = useAppStore((state) => state.setSettings);
  const [draft, setDraft] = useState(stored);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setDraft(stored);
  }, [stored]);

  const patch = useCallback(
    (next: Partial<Settings>) =>
      setDraft((current) => ({ ...current, ...next })),
    [],
  );

  const patchAlerts = useCallback(
    (next: Partial<Settings["alerts"]>) =>
      setDraft((current) => ({
        ...current,
        alerts: { ...current.alerts, ...next },
      })),
    [],
  );

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      setStored(await api.settingsSet(draft));
      notify.success("Settings saved");
    } catch (error) {
      reportError(error);
    } finally {
      setSaving(false);
    }
  }, [draft, setStored]);

  const dirty = JSON.stringify(draft) !== JSON.stringify(stored);

  return (
    <Page
      title="Settings"
      quote="settings"
      actions={
        <Button onClick={handleSave} disabled={saving || !dirty}>
          <Save className="size-4" />
          Save
        </Button>
      }
    >
      <div className="sticky top-2 z-10 h-10">
        {dirty && (
          <button
            type="button"
            className="bg-card text-card-foreground border-border mx-auto flex w-fit cursor-pointer items-center gap-3 rounded-lg border px-4 py-3 text-sm shadow-md"
            onClick={(event) =>
              event.currentTarget
                .closest("main")
                ?.scrollTo({ top: 0, behavior: "smooth" })
            }
          >
            <TriangleAlert className="text-warning size-4" />
            <span className="font-medium">You have unsaved settings.</span>
            <span className="text-muted-foreground">
              Click to scroll to Save.
            </span>
          </button>
        )}
      </div>

      <NotificationChannels draft={draft} patch={patch} />
      <div className="grid items-start gap-6 @7xl:grid-cols-2">
        <FissureAlerts draft={draft} patchAlerts={patchAlerts} />
        <CycleTimers draft={draft} patchAlerts={patchAlerts} />
      </div>
      <PricesAndData draft={draft} patch={patch} />
      <StatsTabSetting draft={draft} patch={patch} />
    </Page>
  );
}
