import { check } from "@tauri-apps/plugin-updater";
import { useCallback, useEffect, useState } from "react";
import { Navigate, Route, Routes } from "react-router";
import { toast } from "sonner";
import { AppSidebar } from "@/components/app-sidebar";
import { EventBridge } from "@/components/event-bridge";
import { OverlayWindow } from "@/components/overlay-window";
import { StartupScreen } from "@/components/startup-screen";
import { StatusBar } from "@/components/status-bar";
import { Toaster } from "@/components/ui/sonner";
import { UpdateDialog } from "@/components/update-dialog";
import { useListen } from "@/hooks/use-listen";
import { api, errorMessage, events, isStarting, logError } from "@/lib/bridge";
import { AboutPage } from "@/pages/about";
import { FoundryPage } from "@/pages/foundry/foundry";
import { InventoryPage } from "@/pages/inventory";
import { MarketPage } from "@/pages/market/market";
import { MasteryPage } from "@/pages/mastery";
import { OverlaysPage } from "@/pages/overlays";
import { RelicPlannerPage } from "@/pages/relic-planner/relic-planner";
import { ResourcesPage } from "@/pages/resources/resources";
import { RivensPage } from "@/pages/rivens/rivens";
import { SettingsPage } from "@/pages/settings/settings";
import { StatsPage } from "@/pages/stats";
import { WorldPage } from "@/pages/world/world";
import { useAppStore } from "@/stores/app-store";

function retryDelay(attempt: number, jitter: number): number {
  const ceiling = Math.min(250 * 2 ** attempt, 4_000);
  return Math.round(ceiling * (0.5 + jitter / 2));
}

function waitForRetry(delay: number, signal: AbortSignal): Promise<void> {
  if (signal.aborted) {
    return Promise.resolve();
  }

  return new Promise((resolve) => {
    const finish = () => {
      window.clearTimeout(timer);
      signal.removeEventListener("abort", finish);
      resolve();
    };
    const timer = window.setTimeout(finish, delay);
    signal.addEventListener("abort", finish, { once: true });
  });
}

function useBoot() {
  const [retrySchedule, setRetrySchedule] = useState(() => ({
    jitter: Math.random(),
  }));
  const [fatalError, setFatalError] = useState<string | null>(null);
  const { setReady, setBootError, setStatus, setWorld, setSettings } =
    useAppStore();

  const retry = useCallback(() => {
    setBootError(null);
    setFatalError(null);
    setRetrySchedule({ jitter: Math.random() });
  }, [setBootError]);

  const fail = useCallback(
    (message: string) => {
      setFatalError(message);
      setReady(false);
      setBootError(message);
      toast.error(message);
    },
    [setBootError, setReady],
  );

  useListen(events.appReady, retry);
  useListen<string>(events.appError, fail);

  useEffect(() => {
    if (fatalError) {
      return;
    }

    const controller = new AbortController();

    async function boot() {
      let attempt = 0;
      let failures = 0;

      while (!controller.signal.aborted) {
        try {
          const [status, world, settings] = await Promise.all([
            api.gameStatus(),
            api.worldstate(),
            api.settingsGet(),
          ]);
          if (controller.signal.aborted) {
            return;
          }

          setStatus(status);
          setWorld(world);
          setSettings(settings);
          setBootError(null);
          setReady(true);
          return;
        } catch (error) {
          if (controller.signal.aborted) {
            return;
          }

          if (!isStarting(error)) {
            failures += 1;
            if (failures >= 5) {
              fail(errorMessage(error));
              return;
            }
          }

          await waitForRetry(
            retryDelay(attempt, retrySchedule.jitter),
            controller.signal,
          );
          attempt += 1;
        }
      }
    }

    boot();
    return () => controller.abort();
  }, [
    fail,
    fatalError,
    retrySchedule,
    setBootError,
    setReady,
    setSettings,
    setStatus,
    setWorld,
  ]);

  return retry;
}

function MainWindow() {
  const retry = useBoot();
  const { ready, bootError, settings, setUpdate } = useAppStore();
  const statsTab = settings?.stats_tab_enabled ?? true;
  const checkForUpdates = settings?.check_for_updates ?? true;

  useEffect(() => {
    if (!ready || !checkForUpdates) {
      return;
    }
    api
      .updatesSupported()
      .then((supported) => (supported ? check() : null))
      .then(setUpdate, (error) => logError("Update check", error));
  }, [ready, checkForUpdates, setUpdate]);

  if (!ready) {
    return (
      <>
        <StartupScreen error={bootError} onRetry={retry} />
        <Toaster position="bottom-right" />
      </>
    );
  }

  return (
    <div className="flex h-full w-full overflow-hidden">
      <AppSidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <StatusBar />
        <main className="min-h-0 flex-1 overflow-y-auto">
          <Routes>
            <Route path="/" element={<Navigate to="/world" replace />} />
            <Route path="/world" element={<WorldPage />} />
            <Route path="/foundry" element={<FoundryPage />} />
            <Route path="/inventory" element={<InventoryPage />} />
            <Route path="/relics" element={<RelicPlannerPage />} />
            <Route path="/rivens" element={<RivensPage />} />
            <Route path="/overlays" element={<OverlaysPage />} />
            <Route path="/mastery" element={<MasteryPage />} />
            <Route path="/resources" element={<ResourcesPage />} />
            <Route path="/market" element={<MarketPage />} />
            <Route
              path="/stats"
              element={
                statsTab ? <StatsPage /> : <Navigate to="/world" replace />
              }
            />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="/about" element={<AboutPage />} />
            <Route path="*" element={<Navigate to="/world" replace />} />
          </Routes>
        </main>
      </div>
      <EventBridge />
      <UpdateDialog />
      <Toaster position="bottom-right" />
    </div>
  );
}

export default function App() {
  return (
    <Routes>
      <Route path="/overlay/*" element={<OverlayWindow />} />
      <Route path="*" element={<MainWindow />} />
    </Routes>
  );
}
