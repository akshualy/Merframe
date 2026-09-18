import { useCallback } from "react";
import { type AsyncData, useAsyncData } from "@/hooks/use-async-data";
import { useListen } from "@/hooks/use-listen";
import { api, events } from "@/lib/bridge";

const loaders = {
  inventory: api.inventoryTab,
  foundry: api.foundryTab,
  resources: api.resourcesTab,
  rivens: api.rivensTab,
  stats: api.statsTab,
  worldstate: api.worldstate,
} as const;

export type CommandKey = keyof typeof loaders;

type Payload<K extends CommandKey> = Awaited<ReturnType<(typeof loaders)[K]>>;

export function useCommand<K extends CommandKey>(
  key: K,
): AsyncData<Payload<K>> {
  const load = useCallback(
    () => (loaders[key] as () => Promise<Payload<K>>)(),
    [key],
  );
  const state = useAsyncData(load);

  useListen(events.inventoryUpdated, state.reload);
  useListen(events.pricesUpdated, state.reload);
  useListen(events.rivenDataUpdated, state.reload);
  useListen(events.appReady, state.reload);

  return state;
}
