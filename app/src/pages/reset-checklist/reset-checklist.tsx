import { ChevronDown } from "lucide-react";
import { OptionTabs } from "@/components/option-tabs";
import { Page } from "@/components/page";
import { CheckboxMenu } from "@/components/ui/checkbox-menu";
import { Hint } from "@/components/ui/hint";
import { SwitchField } from "@/components/ui/switch";
import { useAsyncData } from "@/hooks/use-async-data";
import { useNow } from "@/hooks/use-now";
import { useRollover } from "@/hooks/use-rollover";
import { useTabParam } from "@/hooks/use-tab-param";
import { api, type EventName, reportError } from "@/lib/bridge";
import { toggled } from "@/lib/filters";
import { counted, dateTime } from "@/lib/format";
import { useAppStore, useSettings } from "@/stores/app-store";
import type {
  ResetChecklistGroup,
  ResetChecklistTab,
  ResetWindow,
} from "@/types";
import {
  type CheckResetChecklistRow,
  ResetChecklistSection,
} from "./checklist";
import { ResetChecklistVendorPurchases } from "./shops";
import { ResetChecklistSyndicates } from "./syndicates";

type ResetChecklistPageTab = "checklist" | "shops" | "syndicates";

const PAGE_TABS: { value: ResetChecklistPageTab; label: string }[] = [
  { value: "checklist", label: "Checklist" },
  { value: "shops", label: "Shops" },
  { value: "syndicates", label: "Syndicates" },
];

const RESET_CHECKLIST_RELOAD_EVENTS: readonly EventName[] = [
  "resetChecklistUpdated",
  "inventoryUpdated",
  "worldStateUpdated",
];

const SYNDICATES_RELOAD_EVENTS: readonly EventName[] = ["inventoryUpdated"];

const CHECKLIST_TITLES: Record<ResetChecklistGroup, string> = {
  daily: "Daily",
  sortie: "Mid-Daily Reset",
  weekly: "Weekly",
};

const CHECKLIST_COLUMNS: ResetChecklistGroup[][] = [
  ["daily", "sortie"],
  ["weekly"],
];

function checklistResets(
  tab: ResetChecklistTab,
): Record<ResetChecklistGroup, ResetWindow | null> {
  return { daily: tab.daily, sortie: tab.sortie, weekly: tab.weekly };
}

function syncedNote(syncedAt: string | null) {
  return syncedAt === null
    ? "The inventory has no sync time, so automatic checks may be out of date"
    : `Checks use the inventory synced on ${dateTime(syncedAt)}`;
}

function resetInstants(tab: ResetChecklistTab | null): string[] {
  if (tab === null) {
    return [];
  }

  return [
    tab.daily.expiry,
    tab.weekly.expiry,
    ...(tab.sortie ? [tab.sortie.expiry] : []),
    ...tab.rows.flatMap((row) =>
      row.own_window ? [row.own_window.expiry] : [],
    ),
  ];
}

export function ResetChecklistPage() {
  const settings = useSettings();
  const saveSettings = useAppStore((state) => state.saveSettings);
  const now = useNow();
  const [tab, setTab] = useTabParam<ResetChecklistPageTab>("checklist");
  const checklist = useAsyncData(
    "reset_checklist_tab",
    api.resetChecklistTab,
    RESET_CHECKLIST_RELOAD_EVENTS,
  );
  const { data, error } = checklist;
  const syndicates = useAsyncData(
    "syndicates_tab",
    api.syndicatesTab,
    SYNDICATES_RELOAD_EVENTS,
  );
  const shown = tab === "syndicates" ? syndicates : checklist;

  useRollover(resetInstants(data), now, checklist.reload);
  useRollover(
    syndicates.data ? [syndicates.data.daily_reset.expiry] : [],
    now,
    syndicates.reload,
  );

  const hidden = settings.reset_checklist_hidden;
  const listed = (data?.rows ?? []).filter(
    (row) => settings.reset_checklist_show_locked || !row.locked,
  );
  const open = listed.filter(
    (row) =>
      !hidden.includes(row.id) &&
      !(settings.reset_checklist_hide_done && row.done),
  );
  const hiddenCount = listed.filter((row) => hidden.includes(row.id)).length;

  const check: CheckResetChecklistRow = (id, checked) => {
    api.setResetChecklistChecked(id, checked).catch(reportError);
  };

  return (
    <Page
      title="Reset Checklist"
      quote="resetChecklist"
      actions={
        <>
          {tab === "checklist" && (
            <SwitchField
              id="reset-checklist-hide-done"
              label="Hide done"
              checked={settings.reset_checklist_hide_done}
              onChange={(checked) =>
                saveSettings({ reset_checklist_hide_done: checked })
              }
            />
          )}
          {(tab === "checklist" || tab === "shops") && (
            <SwitchField
              id="reset-checklist-show-locked"
              label="Show locked"
              checked={settings.reset_checklist_show_locked}
              onChange={(checked) =>
                saveSettings({ reset_checklist_show_locked: checked })
              }
            />
          )}
          {tab === "checklist" && (
            <CheckboxMenu
              trigger={
                <>
                  {counted(hiddenCount, "hidden entry", "hidden entries")}
                  <ChevronDown className="size-4" />
                </>
              }
              align="end"
              disabled={data === null || error !== null}
              items={listed.map((row) => ({
                key: row.id,
                label: row.name,
                checked: !hidden.includes(row.id),
                onCheckedChange: (checked) =>
                  saveSettings({
                    reset_checklist_hidden: toggled(hidden, row.id, !checked),
                  }),
              }))}
            />
          )}
        </>
      }
      loading={shown.loading && !shown.data}
      error={shown.error}
    >
      <OptionTabs value={tab} options={PAGE_TABS} onChange={setTab} />
      {tab === "checklist" && data && (
        <>
          <Hint>{syncedNote(data.synced_at)}</Hint>
          <div className="grid items-start gap-4 @5xl:grid-cols-2">
            {CHECKLIST_COLUMNS.map((column) => (
              <div key={column.join()} className="flex flex-col gap-4">
                {column.map((group) => (
                  <ResetChecklistSection
                    key={group}
                    title={CHECKLIST_TITLES[group]}
                    reset={checklistResets(data)[group]}
                    rows={open.filter((row) => row.group === group)}
                    now={now}
                    onCheck={check}
                  />
                ))}
              </div>
            ))}
          </div>
        </>
      )}
      {tab === "shops" && data && (
        <ResetChecklistVendorPurchases
          vendors={data.vendors}
          kuva={data.kuva}
          showLocked={settings.reset_checklist_show_locked}
          reset={data.weekly}
          now={now}
        />
      )}
      {tab === "syndicates" && syndicates.data && (
        <ResetChecklistSyndicates tab={syndicates.data} now={now} />
      )}
    </Page>
  );
}
