export function marketAssetUrl(path: string): string {
  return `https://warframe.market/static/assets/${path}`;
}

export function MarketThumb({ thumb, name }: { thumb?: string; name: string }) {
  if (!thumb) {
    return <span className="bg-muted size-6 shrink-0 rounded-md" />;
  }
  return (
    <img
      src={marketAssetUrl(thumb)}
      alt={name}
      width={24}
      height={24}
      loading="lazy"
      className="bg-muted size-6 shrink-0 rounded-md object-contain"
    />
  );
}
