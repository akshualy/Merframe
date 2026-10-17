export function findOrThrow<T>(
  entries: readonly T[],
  name: string,
  nameOf: (entry: T) => string,
): T {
  const found = entries.find((entry) => nameOf(entry) === name);
  if (found === undefined) {
    throw new Error(`${name} missing from the test data`);
  }

  return found;
}
