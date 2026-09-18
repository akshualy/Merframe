import { BellRing } from "lucide-react";
import { useCallback, useState } from "react";
import { toast } from "sonner";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api, reportError } from "@/lib/bridge";
import type { Settings } from "@/types";
import { CheckboxRow, type Patch } from "./row";

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
      toast.success("Test notification sent");
    } catch (error) {
      reportError(error);
    } finally {
      setTesting(false);
    }
  }, [draft]);

  return (
    <Section
      title="Notification Channels"
      description="In-app toasts always stay on."
      action={
        <Button
          variant="outline"
          size="sm"
          onClick={handleTest}
          disabled={testing}
        >
          <BellRing className="size-4" />
          Test Notifications
        </Button>
      }
    >
      <div className="flex flex-col gap-4">
        <CheckboxRow
          checked={draft.windows_notifications_enabled}
          onChange={(checked) =>
            patch({ windows_notifications_enabled: checked })
          }
        >
          Desktop notifications
        </CheckboxRow>
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
        <CheckboxRow
          checked={draft.discord_notifications_enabled}
          onChange={(checked) =>
            patch({ discord_notifications_enabled: checked })
          }
        >
          Mirror new conversations to Discord
        </CheckboxRow>
        <div className="flex flex-col gap-2">
          <Label htmlFor="webhook">Discord webhook URL</Label>
          <Input
            id="webhook"
            value={draft.discord_webhook ?? ""}
            onChange={(e) => patch({ discord_webhook: e.target.value || null })}
            placeholder="https://discord.com/api/webhooks/"
          />
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="template">
            Discord message, <span className="text-accent">{"{tenno}"}</span> is
            the sender's name
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
    </Section>
  );
}
