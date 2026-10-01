import { toast } from "sonner";
import { api, logError } from "@/lib/bridge";
import { useAppStore } from "@/stores/app-store";
import type { InGameToast } from "@/types";

type Tone = "success" | "info" | "warning";

function show(
  tone: Tone,
  title: string,
  { inGame, ...options }: { description?: string; inGame?: InGameToast } = {},
) {
  const settings = useAppStore.getState().settings;
  if (settings && !settings.toasts_enabled) {
    return;
  }
  toast[tone](title, options);
  if (
    inGame &&
    settings?.toasts_in_game &&
    !settings.toasts_in_game_muted.includes(inGame)
  ) {
    api
      .overlayNotify(title, options.description ?? "")
      .catch((error) =>
        logError("Showing the toast over the game failed", error),
      );
  }
}

export const notify = {
  success: show.bind(null, "success"),
  info: show.bind(null, "info"),
  warning: show.bind(null, "warning"),
};
