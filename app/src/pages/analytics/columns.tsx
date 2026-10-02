import type { ColumnDef } from "@tanstack/react-table";
import { ItemImage } from "@/components/item-image";
import { num } from "@/lib/format";

export function itemColumn<
  T extends { name: string; image_name: string | null },
>(): ColumnDef<T> {
  return {
    accessorKey: "name",
    header: "Item",
    enableHiding: false,
    meta: { wrap: true },
    cell: ({ row }) => (
      <>
        <ItemImage
          imageName={row.original.image_name}
          size={24}
          alt={row.original.name}
        />
        <span className="font-medium">{row.original.name}</span>
      </>
    ),
  };
}

export function totalValueColumn<T extends { value: number }>(): ColumnDef<T> {
  return {
    accessorKey: "value",
    header: "Total value",
    meta: { numeric: true },
    cell: ({ row }) => (
      <span className="text-primary font-bold">{num(row.original.value)}</span>
    ),
  };
}
