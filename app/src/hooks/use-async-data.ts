import { useCallback, useEffect, useState } from "react";
import { useLatestAttempt } from "@/hooks/use-latest-attempt";
import { useListen } from "@/hooks/use-listen";
import type { EventName } from "@/lib/bridge";

export interface AsyncData<T> {
  data: T | null;
  error: string | null;
  loading: boolean;
  reload: () => void;
}

const CACHE_LIMIT = 16;
const cache = new Map<string, unknown>();

function remember(key: string, payload: unknown) {
  cache.delete(key);
  cache.set(key, payload);
  for (const oldest of [...cache.keys()].slice(0, -CACHE_LIMIT)) {
    cache.delete(oldest);
  }
}

export function useAsyncData<T>(
  key: string,
  load: (() => Promise<T>) | null,
  reloadOn: readonly EventName[] = [],
): AsyncData<T> {
  const [data, setData] = useState<T | null>(
    () => (cache.get(key) as T | undefined) ?? null,
  );
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(!cache.has(key));
  const { attempt, cancel } = useLatestAttempt();

  const reload = useCallback(async () => {
    if (!load) {
      return;
    }

    if (cache.has(key)) {
      setData(cache.get(key) as T);
    }

    setLoading(true);
    await attempt(load, {
      loaded: (payload) => {
        remember(key, payload);
        setData(payload);
        setError(null);
      },
      failed: setError,
      settled: () => setLoading(false),
    });
  }, [key, load, attempt]);

  useEffect(() => {
    reload();
    return cancel;
  }, [reload, cancel]);

  useListen(reloadOn, reload);

  return { data, error, loading, reload };
}
