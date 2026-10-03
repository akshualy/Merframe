import { Download, FileSearch, FolderOpen } from "lucide-react";
import { useCallback } from "react";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api, reportError } from "@/lib/bridge";
import { dateTime, num } from "@/lib/format";
import { notify } from "@/lib/toast";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/app-store";
import type { OrderType, Settings, TraderStatus } from "@/types";
import { CheckboxRow, MinutesSlider, type Patch, SwitchRow } from "./row";

const ANY_LANGUAGE = "any";

const TRADER_STATUS: { value: TraderStatus; label: string }[] = [
  { value: "ingame", label: "In game" },
  { value: "online", label: "In game or online" },
  { value: "any", label: "Anyone" },
];

const SET_PART_CLICK: { value: OrderType; label: string }[] = [
  { value: "sell", label: "Sellers, to whisper the cheapest one" },
  { value: "buy", label: "Buyers, to post a buy order" },
];

const TRADER_LANGUAGES: { value: string; label: string }[] = [
  { value: "en", label: "English" },
  { value: "de", label: "German" },
  { value: "es", label: "Spanish" },
  { value: "fr", label: "French" },
  { value: "it", label: "Italian" },
  { value: "ko", label: "Korean" },
  { value: "pl", label: "Polish" },
  { value: "pt", label: "Portuguese" },
  { value: "ru", label: "Russian" },
  { value: "uk", label: "Ukrainian" },
  { value: "zh-hans", label: "Chinese, simplified" },
  { value: "zh-hant", label: "Chinese, traditional" },
];

export function PricesAndData({
  draft,
  patch,
}: {
  draft: Settings;
  patch: Patch;
}) {
  const { status } = useAppStore();

  const handleExport = useCallback(async () => {
    try {
      const dir = await api.exportBundle();
      if (dir) {
        notify.success("Exported", { description: dir });
      }
    } catch (error) {
      reportError(error);
    }
  }, []);

  const handleRefreshPrices = useCallback(async () => {
    try {
      const count = await api.refreshPrices();
      notify.success(
        count === 0
          ? "The price table is already current"
          : `${num(count)} prices loaded`,
      );
    } catch (error) {
      reportError(error);
    }
  }, []);

  const handlePickLogFile = useCallback(async () => {
    try {
      const path = await api.pickLogFile();
      if (path) {
        patch({ log_file_path: path });
      }
    } catch (error) {
      reportError(error);
    }
  }, [patch]);

  const handleOpenDataFolder = useCallback(async () => {
    try {
      await api.openDataFolder();
    } catch (error) {
      reportError(error);
    }
  }, []);

  const handleOpenGameLogFolder = useCallback(async () => {
    try {
      await api.openGameLogFolder();
    } catch (error) {
      reportError(error);
    }
  }, []);

  return (
    <Section title="Prices and Data">
      <div className="grid gap-x-8 gap-y-6 @3xl:grid-cols-2">
        <div className="flex flex-col gap-4">
          <MinutesSlider
            id="market-poll"
            label="warframe.market refresh"
            value={draft.market_poll_minutes}
            min={2}
            max={60}
            step={1}
            onChange={(minutes) => patch({ market_poll_minutes: minutes })}
            hint="Your orders and riven auctions. The market tab refreshes every minute."
          />
          <MinutesSlider
            id="worldstate"
            label="World state refresh"
            value={draft.world_state_interval_minutes}
            min={5}
            max={10}
            step={1}
            onChange={(minutes) =>
              patch({ world_state_interval_minutes: minutes })
            }
          />
          <div className="grid gap-3 @sm:grid-cols-2">
            <div className="flex flex-col gap-2">
              <Label htmlFor="trader-status">Trader order</Label>
              <Select
                value={draft.market_trader_status}
                onValueChange={(value) =>
                  patch({ market_trader_status: value as TraderStatus })
                }
              >
                <SelectTrigger id="trader-status" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {TRADER_STATUS.map(({ value, label }) => (
                    <SelectItem key={value} value={value}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="trader-locale">Trader language</Label>
              <Select
                value={draft.market_trader_locale ?? ANY_LANGUAGE}
                onValueChange={(value) =>
                  patch({
                    market_trader_locale: value === ANY_LANGUAGE ? null : value,
                  })
                }
              >
                <SelectTrigger id="trader-locale" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={ANY_LANGUAGE}>Any language</SelectItem>
                  {TRADER_LANGUAGES.map(({ value, label }) => (
                    <SelectItem key={value} value={value}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
        </div>
        <div className="flex flex-col gap-4">
          <SwitchRow
            checked={draft.market_auto_close}
            onChange={(checked) => patch({ market_auto_close: checked })}
            hint="A traded item marks its sell order sold, a traded riven closes its auction."
          >
            Close the matching listing when a trade completes
          </SwitchRow>
          <SwitchRow
            checked={draft.market_offline_after_last_trade}
            onChange={(checked) =>
              patch({ market_offline_after_last_trade: checked })
            }
            hint="Sets your warframe.market status to offline once the day's trade limit is used up."
          >
            Go offline after the last trade
          </SwitchRow>
          <SwitchRow
            checked={draft.market_hide_after_last_trade}
            onChange={(checked) =>
              patch({ market_hide_after_last_trade: checked })
            }
            hint="Hides all your warframe.market orders once the day's trade limit is used up."
          >
            Hide all listings after the last trade
          </SwitchRow>
          <div className="pl-6">
            <CheckboxRow
              checked={draft.market_hide_auctions_after_last_trade}
              disabled={!draft.market_hide_after_last_trade}
              onChange={(checked) =>
                patch({ market_hide_auctions_after_last_trade: checked })
              }
              hint="Riven auctions are hidden together with the orders."
            >
              Hide auctions too
            </CheckboxRow>
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="set-part-click">Clicking a set part opens</Label>
            <Select
              value={draft.set_part_click}
              onValueChange={(value) =>
                patch({ set_part_click: value as OrderType })
              }
            >
              <SelectTrigger id="set-part-click" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {SET_PART_CLICK.map(({ value, label }) => (
                  <SelectItem key={value} value={value}>
                    {label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <CheckboxRow
            checked={draft.take_rank_into_account}
            onChange={(checked) => patch({ take_rank_into_account: checked })}
            hint="A sell order warns when you own fewer copies than it offers. With this on, only mods at the listed rank count."
          >
            Take the mod rank into account for the missing items check
          </CheckboxRow>
        </div>
        <div className="flex flex-col gap-2 @3xl:col-span-2">
          <span className="text-sm font-medium">EE.log</span>
          <div className="flex flex-wrap items-center gap-3">
            <Button variant="outline" onClick={handlePickLogFile}>
              <FileSearch className="size-4" />
              Choose EE.log
            </Button>
            <Button
              variant="outline"
              disabled={!draft.log_file_path}
              onClick={() => patch({ log_file_path: null })}
            >
              Find It Automatically
            </Button>
          </div>
          <Hint className={cn(!status?.log_file && "italic")}>
            {status?.log_file
              ? `Watching ${status.log_file}`
              : "EE.log not found on this machine"}
          </Hint>
        </div>
        <div className="flex flex-wrap items-center gap-3 @3xl:col-span-2">
          <Button variant="outline" onClick={handleExport}>
            <Download className="size-4" />
            Export Inventory JSON
          </Button>
          <Button variant="outline" onClick={handleRefreshPrices}>
            Refresh Prices Now
          </Button>
          <Button variant="outline" onClick={handleOpenDataFolder}>
            <FolderOpen className="size-4" />
            Open Merframe Data Folder
          </Button>
          <Button
            variant="outline"
            disabled={!status?.log_file}
            onClick={handleOpenGameLogFolder}
          >
            <FolderOpen className="size-4" />
            Open EE.log Folder
          </Button>
        </div>
        <div className="@3xl:col-span-2">
          <SwitchRow
            checked={draft.check_for_updates}
            onChange={(checked) => patch({ check_for_updates: checked })}
            hint="The About page checks on request either way."
          >
            Check for updates on start
          </SwitchRow>
        </div>
        <div className="flex flex-col gap-1 @3xl:col-span-2">
          <Hint className={cn(!status?.price_table_at && "italic")}>
            {status?.price_table_at
              ? `Price table checked ${dateTime(status.price_table_at)}`
              : "Price table not fetched yet"}
          </Hint>
          <Hint>
            Prime parts and set prices are re-checked every 30 minutes, other
            items with orders every 4 hours and the rest daily. Trade volumes
            and the 90-day history refresh once a day.
          </Hint>
        </div>
      </div>
    </Section>
  );
}

export function StatsTabSetting({
  draft,
  patch,
}: {
  draft: Settings;
  patch: Patch;
}) {
  return (
    <Section
      title="Stats"
      description="The Stats page charts the local history Merframe keeps of your account."
    >
      <SwitchRow
        checked={draft.stats_tab_enabled}
        onChange={(checked) => patch({ stats_tab_enabled: checked })}
        hint="The history keeps recording while the tab is hidden."
      >
        Show the Stats tab
      </SwitchRow>
    </Section>
  );
}
