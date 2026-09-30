import { toast } from "sonner";
import { useAppStore } from "@/stores/app-store";

function enabled() {
  return useAppStore.getState().settings?.toasts_enabled ?? true;
}

export const notify = {
  success: (...args: Parameters<typeof toast.success>) => {
    if (enabled()) {
      toast.success(...args);
    }
  },
  info: (...args: Parameters<typeof toast.info>) => {
    if (enabled()) {
      toast.info(...args);
    }
  },
  warning: (...args: Parameters<typeof toast.warning>) => {
    if (enabled()) {
      toast.warning(...args);
    }
  },
};
