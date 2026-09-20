import { Save } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { ErrorNote, Page, Quoted } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { api, errorMessage, isStarting, reportError } from "@/lib/bridge";
import { usePageQuote } from "@/lib/quotes";
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
  const [starting, setStarting] = useState(false);
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
        setStarting(false);
      } catch (error) {
        if (isStarting(error)) {
          setStarting(true);
          return;
        }
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
      toast.success("Settings saved");
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
        {!starting && !error && <Skeleton className="h-96 w-full" />}
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

      <NotificationChannels draft={draft} patch={patch} />
      <div className="grid items-start gap-6 2xl:grid-cols-2">
        <FissureAlerts draft={draft} patchAlerts={patchAlerts} />
        <CycleTimers draft={draft} patchAlerts={patchAlerts} />
      </div>
      <PricesAndData draft={draft} patch={patch} />
      <StatsTabSetting draft={draft} patch={patch} />
    </Page>
  );
}
