import { relaunch } from "@tauri-apps/plugin-process";
import { useCallback, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { reportError } from "@/lib/bridge";
import { useAppStore } from "@/stores/app-store";

export function UpdateDialog() {
  const { update, setUpdate, saveSettings } = useAppStore(
    useShallow((state) => ({
      update: state.update,
      setUpdate: state.setUpdate,
      saveSettings: state.saveSettings,
    })),
  );
  const [progress, setProgress] = useState<number | null>(null);

  const close = useCallback(() => setUpdate(null), [setUpdate]);

  const handleInstall = useCallback(async () => {
    if (!update) {
      return;
    }
    setProgress(0);
    let total = 0;
    let downloaded = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress" && total > 0) {
          downloaded += event.data.chunkLength;
          setProgress((downloaded / total) * 100);
        }
      });
      await relaunch();
    } catch (error) {
      reportError(error);
      setProgress(null);
    }
  }, [update]);

  const handleMute = useCallback(async () => {
    const failure = await saveSettings({
      check_for_updates: false,
    });
    if (failure === null) {
      close();
    }
  }, [saveSettings, close]);

  const installing = progress !== null;

  return (
    <Dialog
      open={update !== null}
      onOpenChange={(open) => !open && !installing && close()}
    >
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Merframe {update?.version} Available</DialogTitle>
          <DialogDescription>
            You are on {update?.currentVersion}. The update downloads, installs
            and restarts Merframe.
          </DialogDescription>
        </DialogHeader>
        {installing && <Progress value={progress} />}
        <DialogFooter>
          <Button variant="outline" disabled={installing} onClick={handleMute}>
            Don't Remind Me Again
          </Button>
          <Button variant="outline" disabled={installing} onClick={close}>
            Later
          </Button>
          <Button disabled={installing} onClick={handleInstall}>
            {installing ? "Downloading" : "Update"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
