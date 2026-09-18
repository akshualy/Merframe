import { Download } from "lucide-react";
import { useCallback } from "react";
import { toast } from "sonner";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api, reportError } from "@/lib/bridge";
import { dateTime, num } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/app-store";
import type { Settings } from "@/types";
import { CheckboxRow, type Patch } from "./row";

function MinutesField({
  id,
  label,
  value,
  min,
  max,
  fallback,
  onChange,
  hint,
}: {
  id: string;
  label: string;
  value: number;
  min: number;
  max: number;
  fallback: number;
  onChange: (minutes: number) => void;
  hint?: string;
}) {
  return (
    <div className="flex max-w-64 flex-col gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        type="number"
        min={min}
        max={max}
        value={value}
        onChange={(e) =>
          onChange(
            Math.min(max, Math.max(min, Number(e.target.value) || fallback)),
          )
        }
      />
      {hint && <Hint>{hint}</Hint>}
    </div>
  );
}

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
        toast.success("Exported", { description: dir });
      }
    } catch (error) {
      reportError(error);
    }
  }, []);

  const handleRefreshPrices = useCallback(async () => {
    try {
      const count = await api.refreshPrices();
      toast.success(
        count === 0
          ? "The price table is already current"
          : `${num(count)} prices loaded`,
      );
    } catch (error) {
      reportError(error);
    }
  }, []);

  return (
    <Section title="Prices and Data">
      <div className="flex flex-col gap-4">
        <MinutesField
          id="ttl"
          label="Price refresh in minutes"
          value={draft.price_ttl_minutes}
          min={5}
          max={120}
          fallback={15}
          onChange={(minutes) => patch({ price_ttl_minutes: minutes })}
        />
        <MinutesField
          id="market-poll"
          label="warframe.market refresh in minutes"
          value={draft.market_poll_minutes}
          min={2}
          max={60}
          fallback={5}
          onChange={(minutes) => patch({ market_poll_minutes: minutes })}
          hint="Your orders and riven auctions, while signed in. The market page itself refreshes every minute."
        />
        <CheckboxRow
          checked={draft.market_auto_close}
          onChange={(checked) => patch({ market_auto_close: checked })}
          hint="A traded item marks its sell order sold, a traded riven closes its auction."
        >
          Close the matching listing when a trade completes
        </CheckboxRow>
        <CheckboxRow
          checked={draft.take_rank_into_account}
          onChange={(checked) => patch({ take_rank_into_account: checked })}
          hint="A sell order warns when you own fewer copies than it offers. With this on, only mods at the listed rank count."
        >
          Take the mod rank into account for the missing items check
        </CheckboxRow>
        <MinutesField
          id="worldstate"
          label="World state refresh in minutes"
          value={draft.world_state_interval_minutes}
          min={5}
          max={10}
          fallback={5}
          onChange={(minutes) =>
            patch({ world_state_interval_minutes: minutes })
          }
        />
        <CheckboxRow
          checked={draft.show_full_inventory}
          onChange={(checked) => patch({ show_full_inventory: checked })}
          hint="Otherwise a tab stops at 300 rows until you ask for the rest."
        >
          Show every inventory row
        </CheckboxRow>
        <div className="flex flex-wrap items-center gap-3">
          <Button variant="outline" onClick={handleExport}>
            <Download className="size-4" />
            Export Inventory JSON
          </Button>
          <Button variant="outline" onClick={handleRefreshPrices}>
            Refresh Prices Now
          </Button>
        </div>
        <Hint className={cn(!status?.price_table_at && "italic")}>
          {status?.price_table_at
            ? `Price table checked ${dateTime(status.price_table_at)}`
            : "Price table not fetched yet"}
        </Hint>
        <Hint className={cn(!status?.log_file && "italic")}>
          {status?.log_file
            ? `Watching ${status.log_file}`
            : "EE.log not found on this machine"}
        </Hint>
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
      <CheckboxRow
        checked={draft.stats_tab_enabled}
        onChange={(checked) => patch({ stats_tab_enabled: checked })}
        hint="The history keeps recording while the tab is hidden."
      >
        Show the Stats tab
      </CheckboxRow>
    </Section>
  );
}
