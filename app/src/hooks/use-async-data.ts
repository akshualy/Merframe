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

export function useAsyncData<T>(
  load: (() => Promise<T>) | null,
  reloadOn: readonly EventName[] = [],
): AsyncData<T> {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const newest = useRef(0);

  const reload = useCallback(async () => {
    if (!load) {
      return;
    }
    newest.current += 1;
    const attempt = newest.current;
    const stale = () => attempt !== newest.current;
    setLoading(true);
    try {
      const payload = await load();
      if (stale()) {
        return;
      }
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
  }, [load]);

  useEffect(() => {
    reload();
    return () => {
      newest.current += 1;
    };
  }, [reload]);

  useListen(reloadOn, reload);

  return { data, error, loading, reload };
}
