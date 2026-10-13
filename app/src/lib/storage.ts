function storage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export function readStored(key: string): string | null {
  return storage()?.getItem(key) ?? null;
}

export function parseStoredJson<T>(raw: string): T | null {
  try {
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

export function readStoredJson<T>(key: string): T | null {
  const raw = readStored(key);
  return raw === null ? null : parseStoredJson<T>(raw);
}

export function writeStoredJson(key: string, value: unknown) {
  storage()?.setItem(key, JSON.stringify(value));
}
