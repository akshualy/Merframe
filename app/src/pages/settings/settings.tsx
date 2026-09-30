import { Save, TriangleAlert } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { ErrorNote, Page, Quoted } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { api, errorMessage, reportError } from "@/lib/bridge";
import { usePageQuote } from "@/lib/quotes";
import { notify } from "@/lib/toast";
import { useAppStore } from "@/stores/app-store";
import type { Settings } from "@/types";
import { PricesAndData, StatsTabSetting } from "./data";
import { FissureAlerts } from "./fissures";
import { NotificationChannels } from "./notifications";
import { CycleTimers } from "./timers";

export function SettingsPage() {
  const quote = usePageQuote("settings");
  const { settings: stored, setSettings: setStored } = useAppStore();
  const [draft, setDraft] = useState<Settings | null>(stored);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (stored) {
      setDraft(stored);
    }
  }, [stored]);

  useEffect(() => {
    if (stored) {
      return;
    }
    async function load() {
      try {
        const next = await api.settingsGet();
        setStored(next);
        setDraft(next);
      } catch (error) {
        setError(errorMessage(error));
      }
    }
    load();
  }, [stored, setStored]);

  const patch = useCallback(
    (next: Partial<Settings>) =>
      setDraft((current) => current && { ...current, ...next }),
    [],
  );

  const patchAlerts = useCallback(
    (next: Partial<Settings["alerts"]>) =>
      setDraft(
        (current) =>
          current && { ...current, alerts: { ...current.alerts, ...next } },
      ),
    [],
  );

  const handleSave = useCallback(async () => {
    if (!draft) {
      return;
    }
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

  if (!draft) {
    return (
      <Page title="Settings" description={<Quoted quote={quote} />}>
        {error && <ErrorNote message={error} />}
        {!error && <Skeleton className="h-96 w-full" />}
      </Page>
    );
  }

  return (
    <Page
      title="Settings"
      description={<Quoted quote={quote} />}
      actions={
        <Button onClick={handleSave} disabled={saving || !dirty}>
          <Save className="size-4" />
          Save
        </Button>
      }
    >
      {error && <ErrorNote message={error} />}
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
