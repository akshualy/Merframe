function storage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export function readStoredJson<T>(key: string): T | null {
  const raw = storage()?.getItem(key);
  if (raw == null) {
    return null;
  }
  try {
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

export function writeStoredJson(key: string, value: unknown) {
  storage()?.setItem(key, JSON.stringify(value));
}
