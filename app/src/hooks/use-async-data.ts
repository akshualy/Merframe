import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { errorMessage, isStarting } from "@/lib/bridge";

export interface AsyncData<T> {
  data: T | null;
  error: string | null;
  starting: boolean;
  loading: boolean;
  reload: () => void;
}

export function useAsyncData<T>(load: (() => Promise<T>) | null): AsyncData<T> {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);
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
      setStarting(false);
    } catch (error) {
      if (stale()) {
        return;
      }
      if (isStarting(error)) {
        setStarting(true);
        setError(null);
        return;
      }
      setStarting(false);
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

  return { data, error, starting, loading, reload };
}
