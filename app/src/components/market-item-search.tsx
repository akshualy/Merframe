import { useMemo, useState } from "react";
import { ItemImage } from "@/components/item-image";
import { SearchInput } from "@/components/search-input";
import { useMarketItemsStore } from "@/stores/market-items-store";
import type { MarketItem } from "@/types";

export function ItemSearch({ onPick }: { onPick: (item: MarketItem) => void }) {
  const items = useMarketItemsStore((state) => state.items);
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);

  const matches = useMemo(() => {
    const search = query.trim().toLowerCase();
    if (!search) {
      return [];
    }
    return items
      .filter((item) => item.name.toLowerCase().includes(search))
      .sort(
        (a, b) =>
          Number(b.name.toLowerCase() === search) -
          Number(a.name.toLowerCase() === search),
      )
      .slice(0, 20);
  }, [items, query]);

  return (
    <div className="relative">
      <SearchInput
        value={query}
        onValueChange={(next) => {
          setQuery(next);
          setOpen(true);
        }}
        onFocus={() => setOpen(query.trim().length > 0)}
        onBlur={() => setOpen(false)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            setOpen(false);
          }
        }}
        placeholder="Search warframe.market items"
        className="w-full"
      />
      {open && matches.length > 0 && (
        <ul
          onMouseDown={(e) => e.preventDefault()}
          className="bg-popover absolute top-full z-20 mt-1 max-h-80 w-full overflow-y-auto rounded-md border shadow-md"
        >
          {matches.map((item) => (
            <li key={item.id}>
              <button
                type="button"
                onClick={() => {
                  setQuery("");
                  setOpen(false);
                  onPick(item);
                }}
                className="hover:bg-secondary flex w-full cursor-pointer items-center gap-2 px-3 py-1.5 text-left text-sm"
              >
                <ItemImage
                  imageName={item.image_name}
                  size={24}
                  alt={item.name}
                />
                <span className="truncate">{item.name}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
