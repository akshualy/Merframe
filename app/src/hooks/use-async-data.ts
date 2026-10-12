import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { useListen } from "@/hooks/use-listen";
import { type EventName, errorMessage } from "@/lib/bridge";

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
  const newest = useRef(0);

  const reload = useCallback(async () => {
    if (!load) {
      return;
    }
    newest.current += 1;
    const attempt = newest.current;
    const stale = () => attempt !== newest.current;

    if (cache.has(key)) {
      setData(cache.get(key) as T);
    }
    setLoading(true);
    try {
      const payload = await load();
      if (stale()) {
        return;
      }
      remember(key, payload);
      setData(payload);
      setError(null);
    } catch (error) {
      if (stale()) {
        return;
      }
      const message = errorMessage(error);
      setError(message);
      toast.error(message);
    } finally {
      if (!stale()) {
        setLoading(false);
      }
    }
  }, [key, load]);

  useEffect(() => {
    reload();
    return () => {
      newest.current += 1;
    };
  }, [reload]);

  useListen(reloadOn, reload);

  return { data, error, loading, reload };
}
