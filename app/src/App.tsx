import { check } from "@tauri-apps/plugin-updater";
import { useCallback, useEffect, useRef } from "react";
import { Navigate, Route, Routes } from "react-router";
import { toast } from "sonner";
import { AppSidebar } from "@/components/app-sidebar";
import { EventBridge } from "@/components/event-bridge";
import { MarketPanel } from "@/components/market-panel";
import { OverlayWindow } from "@/components/overlay-window";
import { StartupScreen } from "@/components/startup-screen";
import { StatusBar } from "@/components/status-bar";
import { Toaster } from "@/components/ui/sonner";
import { UpdateDialog } from "@/components/update-dialog";
import { api, errorMessage, logError } from "@/lib/bridge";
import { AboutPage } from "@/pages/about";
import { AnalyticsPage } from "@/pages/analytics/analytics";
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
import { useMarketPanelStore } from "@/stores/market-panel-store";

function useBoot() {
  const newest = useRef(0);
  const { setReady, setBootError, setStatus, setWorld, setSettings } =
    useAppStore();

  const boot = useCallback(async () => {
    newest.current += 1;
    const attempt = newest.current;
    setBootError(null);
    try {
      const [status, world, settings] = await Promise.all([
        api.gameStatus(),
        api.worldstate(),
        api.settingsGet(),
      ]);
      if (attempt !== newest.current) {
        return;
      }
      setStatus(status);
      setWorld(world);
      setSettings(settings);
      setReady(true);
    } catch (error) {
      if (attempt !== newest.current) {
        return;
      }
      const message = errorMessage(error);
      setBootError(message);
      toast.error(message);
    }
  }, [setBootError, setReady, setSettings, setStatus, setWorld]);

  useEffect(() => {
    boot();
    return () => {
      newest.current += 1;
    };
  }, [boot]);

  return boot;
}

function MainWindow() {
  const retry = useBoot();
  const { ready, bootError, settings, setUpdate } = useAppStore();
  const setItems = useMarketPanelStore((state) => state.setItems);
  const statsTab = settings?.stats_tab_enabled ?? true;
  const checkForUpdates = settings?.check_for_updates ?? true;

  useEffect(() => {
    if (!ready) {
      return;
    }
    api
      .marketItems()
      .then(setItems, (error) => logError("Market items", error));
  }, [ready, setItems]);

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
        <Toaster />
      </>
    );
  }

  return (
    <div className="flex h-full w-full overflow-hidden">
      <AppSidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <StatusBar />
        <main className="min-h-0 flex-1 overflow-y-auto [scrollbar-gutter:stable]">
          <div className="@container">
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
              <Route path="/analytics" element={<AnalyticsPage />} />
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
          </div>
        </main>
      </div>
      <MarketPanel />
      <EventBridge />
      <UpdateDialog />
      <Toaster />
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
