import { Download } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { Amount } from "@/components/currency";
import { Figure } from "@/components/figure";
import { InventoryTabView } from "@/components/inventory-tab-view";
import { Page, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAsyncData } from "@/hooks/use-async-data";
import { useInventoryView } from "@/hooks/use-inventory-view";
import { useTabParam } from "@/hooks/use-tab-param";
import { api, reportError } from "@/lib/bridge";
import { num } from "@/lib/format";
import {
  ducats,
  entriesExceptSets,
  INVENTORY_TABS,
  type InventoryTabKey,
  worth,
} from "@/lib/inventory-entries";
import { notify } from "@/lib/toast";

const TAB_LABELS: Record<InventoryTabKey, string> = {
  parts: "All Parts",
  relics: "Relics",
  mods: "Mods",
  arcanes: "Arcanes",
  misc: "Misc",
  sets: "Sets",
};

export function InventoryPage() {
  const { data, error, loading } = useAsyncData(
    "inventory_tab",
    api.inventoryTab,
    ["inventoryUpdated", "pricesUpdated", "favouriteUpdated", "marketUpdated"],
  );
  const [exporting, setExporting] = useState(false);
  const [tab, setTab] = useTabParam<InventoryTabKey>("parts");

  const { entries, flags, filters, updateFilters, visible } = useInventoryView(
    tab,
    data,
  );
  const pageWorth = useMemo(() => worth(visible), [visible]);
  const accountPlat = useMemo(
    () => (data ? worth(entriesExceptSets(data)).plat : 0),
    [data],
  );
  const showsDucats =
    tab !== "sets" && entries.some((entry) => ducats(entry) !== null);

  const handleExport = useCallback(async () => {
    setExporting(true);
    try {
      const dir = await api.exportBundle();
      if (dir) {
        notify.success("Exported", { description: dir });
      }
    } catch (error) {
      reportError(error);
    } finally {
      setExporting(false);
    }
  }, []);

  return (
    <Page
      title="Inventory"
      quote="inventory"
      actions={
        <Button variant="outline" onClick={handleExport} pending={exporting}>
          <Download className="size-4" />
          Export JSON
        </Button>
      }
      loading={loading && !data}
      error={error}
    >
      <Section>
        <Tabs value={tab} onValueChange={setTab}>
          <div className="mb-4 flex flex-wrap items-end justify-between gap-4">
            <TabsList>
              {INVENTORY_TABS.map((key) => (
                <TabsTrigger key={key} value={key}>
                  {TAB_LABELS[key]} ({num(data?.[key].length ?? 0)})
                </TabsTrigger>
              ))}
            </TabsList>
            <div className="flex flex-wrap items-center gap-x-6 gap-y-1">
              <Figure
                label="This tab"
                title="Total of the rows shown"
                value={
                  <span className="flex items-center gap-3">
                    <Amount currency="platinum">{num(pageWorth.plat)}</Amount>
                    {showsDucats && (
                      <Amount currency="ducats">{num(pageWorth.ducats)}</Amount>
                    )}
                  </span>
                }
              />
              <Figure
                label="Account"
                title="Every tab but Sets, which holds the same parts"
                value={<Amount currency="platinum">{num(accountPlat)}</Amount>}
              />
            </div>
          </div>

          <TabsContent value={tab}>
            <InventoryTabView
              key={tab}
              tab={tab}
              entryCount={entries.length}
              visible={visible}
              unpriced={pageWorth.unpriced}
              flags={flags}
              filters={filters}
              updateFilters={updateFilters}
            />
          </TabsContent>
        </Tabs>
      </Section>
    </Page>
  );
}
