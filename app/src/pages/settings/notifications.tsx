import { BellRing } from "lucide-react";
import { useCallback, useState } from "react";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api, reportError } from "@/lib/bridge";
import { notify } from "@/lib/toast";
import type { Settings, ToastPosition } from "@/types";
import { CheckboxRow, type Patch, SwitchRow } from "./row";

const TOAST_POSITIONS: { value: ToastPosition; label: string }[] = [
  { value: "top_left", label: "Top left" },
  { value: "top_centre", label: "Top centre" },
  { value: "top_right", label: "Top right" },
  { value: "bottom_left", label: "Bottom left" },
  { value: "bottom_centre", label: "Bottom centre" },
  { value: "bottom_right", label: "Bottom right" },
];

export function NotificationChannels({
  draft,
  patch,
}: {
  draft: Settings;
  patch: Patch;
}) {
  const [testing, setTesting] = useState(false);
  const handleTest = useCallback(async () => {
    setTesting(true);
    try {
      await api.testNotifications(draft);
      notify.success("Test notification sent");
    } catch (error) {
      reportError(error);
    } finally {
      setTesting(false);
    }
  }, [draft]);

  return (
    <Section title="Notification Channels">
      <div className="grid gap-x-8 gap-y-6 @3xl:grid-cols-2">
        <div className="flex flex-col gap-4">
          <SwitchRow
            checked={draft.toasts_enabled}
            onChange={(checked) => patch({ toasts_enabled: checked })}
            hint="Errors are always shown."
          >
            In-app toasts for confirmations and alerts
          </SwitchRow>
          <div className="flex flex-col gap-2">
            <Label htmlFor="toast-position">Toast position</Label>
            <Select
              value={draft.toast_position}
              onValueChange={(value) =>
                patch({ toast_position: value as ToastPosition })
              }
            >
              <SelectTrigger id="toast-position" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {TOAST_POSITIONS.map(({ value, label }) => (
                  <SelectItem key={value} value={value}>
                    {label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <SwitchRow
            checked={draft.windows_notifications_enabled}
            onChange={(checked) =>
              patch({ windows_notifications_enabled: checked })
            }
          >
            Desktop notifications
          </SwitchRow>
          <CheckboxRow
            checked={draft.sound_notifications_enabled}
            disabled={!draft.windows_notifications_enabled}
            onChange={(checked) =>
              patch({ sound_notifications_enabled: checked })
            }
            hint="Uses your desktop's notification sound."
          >
            Play a sound with them
          </CheckboxRow>
          <CheckboxRow
            checked={draft.notification_only_background}
            onChange={(checked) =>
              patch({ notification_only_background: checked })
            }
          >
            Only notify while Warframe is in the background
          </CheckboxRow>
          <Button
            variant="outline"
            size="sm"
            className="self-start"
            onClick={handleTest}
            disabled={testing}
          >
            <BellRing className="size-4" />
            Test Notifications
          </Button>
        </div>
        <div className="flex flex-col gap-4">
          <SwitchRow
            checked={draft.discord_notifications_enabled}
            onChange={(checked) =>
              patch({ discord_notifications_enabled: checked })
            }
          >
            Mirror new conversations to Discord
          </SwitchRow>
          <SwitchRow
            checked={draft.discord_fissure_alerts}
            onChange={(checked) => patch({ discord_fissure_alerts: checked })}
          >
            Mirror fissure alerts to Discord
          </SwitchRow>
          <SwitchRow
            checked={draft.discord_timer_alerts}
            onChange={(checked) => patch({ discord_timer_alerts: checked })}
          >
            Mirror cycle timers to Discord
          </SwitchRow>
          <div className="flex flex-col gap-2">
            <Label htmlFor="webhook">Discord webhook URL</Label>
            <Input
              id="webhook"
              value={draft.discord_webhook ?? ""}
              onChange={(e) =>
                patch({ discord_webhook: e.target.value || null })
              }
              placeholder="https://discord.com/api/webhooks/"
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="template">
              Conversation notification,{" "}
              <span className="text-accent">{"{tenno}"}</span> is the sender's
              name
            </Label>
            <Input
              id="template"
              value={draft.discord_message_template}
              onChange={(e) =>
                patch({ discord_message_template: e.target.value })
              }
            />
          </div>
        </div>
      </div>
    </Section>
  );
}
