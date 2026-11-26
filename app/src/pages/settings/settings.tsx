import { Save, TriangleAlert, Undo2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useLocation } from "react-router";
import { Page } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useTabParam } from "@/hooks/use-tab-param";
import { api, reportError } from "@/lib/bridge";
import { notify } from "@/lib/toast";
import { scrollToSection } from "@/lib/utils";
import { useAppStore, useSettings } from "@/stores/app-store";
import type { Settings } from "@/types";
import { ArbitrationAlertSettingsSection } from "./arbitrations";
import { DangerZone } from "./danger-zone";
import { PricesAndData, StatsTabSetting, UpdateCheckSetting } from "./data";
import { DisplaySetting } from "./display";
import { EventFeed } from "./feed";
import { FissureAlerts } from "./fissures";
import { NotificationChannels } from "./notifications";
import { WorldRewardAlertSettings } from "./rewards";
import { SaleSafetySetting } from "./sale-safety";
import { CycleTimers } from "./timers";

type SettingsTab = "general" | "market" | "notifications" | "alerts";

const TABS: { value: SettingsTab; label: string }[] = [
  { value: "general", label: "General" },
  { value: "market", label: "Market and Data" },
  { value: "notifications", label: "Notifications" },
  { value: "alerts", label: "Alerts" },
];

const TAB_CONTENT = "flex w-full flex-none flex-col gap-6";

export function SettingsPage() {
  const stored = useSettings();
  const setStored = useAppStore((state) => state.setSettings);
  const [draft, setDraft] = useState(stored);
  const [saving, setSaving] = useState(false);
  const [tab, setTab] = useTabParam<SettingsTab>("general");

  useEffect(() => {
    setDraft(stored);
  }, [stored]);

  const { hash } = useLocation();
  useEffect(() => {
    if (hash) {
      scrollToSection(hash.slice(1));
    }
  }, [hash]);

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
    <Page title="Settings" quote="settings">
      <Tabs
        className="flex-row flex-wrap items-center gap-6"
        value={tab}
        onValueChange={setTab}
      >
        <TabsList>
          {TABS.map(({ value, label }) => (
            <TabsTrigger key={value} value={value}>
              {label}
            </TabsTrigger>
          ))}
        </TabsList>
        <div className="sticky top-2 z-10 h-13 flex-1 basis-md">
          {dirty && (
            <div className="bg-card text-card-foreground border-border ml-auto flex w-fit items-center gap-3 rounded-lg border py-2 pr-2 pl-4 text-sm shadow-md">
              <TriangleAlert className="text-warning size-4" />
              <span className="font-medium">You have unsaved settings.</span>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setDraft(stored)}
                disabled={saving}
              >
                <Undo2 className="size-4" />
                Discard
              </Button>
              <Button size="sm" onClick={handleSave} pending={saving}>
                <Save className="size-4" />
                Save
              </Button>
            </div>
          )}
        </div>
        <TabsContent value="general" className={TAB_CONTENT}>
          <DisplaySetting draft={draft} patch={patch} />
          <StatsTabSetting draft={draft} patch={patch} />
          <UpdateCheckSetting draft={draft} patch={patch} />
          <EventFeed draft={draft} patch={patch} />
          <DangerZone />
        </TabsContent>
        <TabsContent value="market" className={TAB_CONTENT}>
          <PricesAndData draft={draft} patch={patch} />
          <SaleSafetySetting draft={draft} patch={patch} />
        </TabsContent>
        <TabsContent value="notifications" className={TAB_CONTENT}>
          <NotificationChannels draft={draft} patch={patch} />
        </TabsContent>
        <TabsContent
          value="alerts"
          className="grid w-full flex-none items-start gap-6 @7xl:grid-cols-2"
        >
          <FissureAlerts draft={draft} patchAlerts={patchAlerts} />
          <CycleTimers draft={draft} patchAlerts={patchAlerts} />
          <WorldRewardAlertSettings draft={draft} patchAlerts={patchAlerts} />
          <ArbitrationAlertSettingsSection
            draft={draft}
            patchAlerts={patchAlerts}
          />
        </TabsContent>
      </Tabs>
    </Page>
  );
}
