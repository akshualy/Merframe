import { Stat } from "@/components/page";
import { Switch } from "@/components/ui/switch";
import { api, reportError } from "@/lib/bridge";
import { num } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/app-store";

export function TradesLeft({ count }: { count: number }) {
  const { settings, setSettings } = useAppStore();

  const setOffline = async (checked: boolean) => {
    if (!settings) {
      return;
    }
    const merged = { ...settings, market_offline_after_last_trade: checked };
    setSettings(merged);
    try {
      await api.settingsSet(merged);
    } catch (error) {
      reportError(error);
    }
  };

  return (
    <Stat
      label="Trades left today"
      value={
        <span className={cn(count === 0 && "text-warning")}>{num(count)}</span>
      }
      hint={
        <label className="flex cursor-pointer justify-end items-center gap-2">
          Go offline after the last trade
          <Switch
            checked={settings?.market_offline_after_last_trade ?? false}
            disabled={!settings}
            onCheckedChange={setOffline}
          />
        </label>
      }
    />
  );
}
