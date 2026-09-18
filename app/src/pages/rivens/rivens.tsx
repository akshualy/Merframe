import { useCallback, useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router";
import { DataTable } from "@/components/data-table";
import { FilterGrid } from "@/components/filter-grid";
import { FilterSelect } from "@/components/filter-select";
import { GoodRollAttribution } from "@/components/good-roll";
import { prefetchImages } from "@/components/item-image";
import {
  EmptyPanel,
  ErrorNote,
  Page,
  Quoted,
  Section,
  TableSkeleton,
} from "@/components/page";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useCommand } from "@/hooks/use-command";
import { useListen } from "@/hooks/use-listen";
import { events } from "@/lib/bridge";
import { num } from "@/lib/format";
import { usePageQuote } from "@/lib/quotes";
import type { RivenRow } from "@/types";
import { rivenColumns } from "./columns";
import { RivenDetails } from "./details";
import { RivenListingDialog } from "./listing-dialog";
import { VeiledChallenge } from "./veiled";

export function RivensPage() {
  const quote = usePageQuote("rivens");
  const { data, error, loading, reload } = useCommand("rivens");
  const [params, setParams] = useSearchParams();
  const tab = params.get("tab") ?? "unveiled";
  const [listing, setListing] = useState<RivenRow | null>(null);
  const [weaponClass, setWeaponClass] = useState<string | null>(null);

  useListen(events.marketUpdated, reload);

  useEffect(() => {
    if (!data) {
      return;
    }
    prefetchImages([
      ...data.unveiled.map((row) => row.image_name),
      ...data.veiled.flatMap((group) =>
        group.rivens.map((riven) => riven.image_name),
      ),
    ]);
  }, [data]);

  const classes = useMemo(() => {
    const found = new Set<string>();
    for (const row of data?.unveiled ?? []) {
      if (row.weapon_class) {
        found.add(row.weapon_class);
      }
    }
    return [...found].sort();
  }, [data?.unveiled]);

  const unveiled = useMemo(
    () =>
      (data?.unveiled ?? []).filter(
        (row) => weaponClass === null || row.weapon_class === weaponClass,
      ),
    [data?.unveiled, weaponClass],
  );

  const veiledCount = (data?.veiled ?? []).reduce(
    (total, group) => total + group.count,
    0,
  );

  const columns = useMemo(() => rivenColumns(setListing), []);
  const handleCloseListing = useCallback(() => setListing(null), []);

  if (loading && !data) {
    return (
      <Page title="Riven Explorer" description={<Quoted quote={quote} />}>
        <TableSkeleton />
      </Page>
    );
  }

  return (
    <Page title="Riven Explorer" description={<Quoted quote={quote} />}>
      {error && <ErrorNote message={error} />}

      <Section>
        <Tabs
          value={tab}
          onValueChange={(value) =>
            setParams({ tab: value }, { replace: true })
          }
        >
          <TabsList className="mb-4">
            <TabsTrigger value="unveiled">
              Unveiled ({num(data?.unveiled.length ?? 0)})
            </TabsTrigger>
            <TabsTrigger value="veiled">
              Veiled ({num(veiledCount)})
            </TabsTrigger>
          </TabsList>
          <TabsContent value="unveiled">
            <DataTable
              tableId="rivens"
              columns={columns}
              data={unveiled}
              searchPlaceholder="Filter rivens"
              searchValue={(row) =>
                [
                  row.weapon ?? "",
                  row.name ?? "",
                  ...row.attributes.map(
                    (attribute) => attribute.name ?? attribute.tag,
                  ),
                ].join(" ")
              }
              initialSorting={[{ id: "grade", desc: true }]}
              rowKey={(row) => row.item_id}
              filters={
                classes.length > 1 && (
                  <FilterGrid
                    activeFilters={weaponClass === null ? 0 : 1}
                    onClear={() => setWeaponClass(null)}
                  >
                    <FilterSelect
                      label="Weapon class"
                      value={weaponClass}
                      options={classes.map((value) => ({
                        value,
                        label: value,
                      }))}
                      onChange={setWeaponClass}
                    />
                  </FilterGrid>
                )
              }
              renderSubRow={(row) => <RivenDetails riven={row} />}
            />
            <GoodRollAttribution attribution={data?.attribution ?? null} />
          </TabsContent>
          <TabsContent value="veiled">
            {data && data.veiled.length > 0 ? (
              <div className="flex flex-col gap-3">
                {data.veiled.map((group) => (
                  <VeiledChallenge key={group.challenge_id} group={group} />
                ))}
              </div>
            ) : (
              <EmptyPanel>No veiled rivens in the inventory.</EmptyPanel>
            )}
          </TabsContent>
        </Tabs>
      </Section>

      {listing && (
        <RivenListingDialog
          key={listing.item_id}
          riven={listing}
          onClose={handleCloseListing}
        />
      )}
    </Page>
  );
}
