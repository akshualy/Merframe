import { Download } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { Amount } from "@/components/currency";
import { Figure } from "@/components/figure";
import { InventoryTabView } from "@/components/inventory-tab-view";
import { Page, Section } from "@/components/page";
import { useSaleSafetySettings } from "@/components/sale-safety";
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
  inventoryTotals,
  listableTotals,
} from "@/lib/inventory-entries";
import { notify } from "@/lib/toast";

const LIQUIDATION_NOTE =
  "Copies beyond what a quarter of the last 30 days of trades could absorb count at the buy price, or nothing without a buyer";

const TAB_LABELS: Record<InventoryTabKey, string> = {
  parts: "All Parts",
  relics: "Relics",
  mods: "Mods",
  arcanes: "Arcanes",
  misc: "Misc",
  sets: "Sets",
};

export function InventoryPage() {
  const saleSafety = useSaleSafetySettings();
  const load = useCallback(() => api.inventoryTab(saleSafety), [saleSafety]);
  const { data, error, loading } = useAsyncData(
    `inventory_tab:${JSON.stringify(saleSafety)}`,
    load,
    ["inventoryUpdated", "pricesUpdated", "favouriteUpdated", "marketUpdated"],
  );
  const [exporting, setExporting] = useState(false);
  const [tab, setTab] = useTabParam<InventoryTabKey>("parts");

  const { entries, flags, filters, updateFilters, visible, listable } =
    useInventoryView(tab, data);
  const pageTotals = useMemo(
    () => (listable ? listableTotals(visible) : inventoryTotals(visible)),
    [visible, listable],
  );
  const pageScope = listable
    ? "Copies safe to list in the rows shown"
    : "Rows shown";
  const accountValue = useMemo(
    () =>
      data ? inventoryTotals(entriesExceptSets(data)).liquidationValue : 0,
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
                title={`${pageScope}. ${LIQUIDATION_NOTE}`}
                value={
                  <span className="flex items-center gap-3">
                    <Amount currency="platinum">
                      {num(pageTotals.liquidationValue)}
                    </Amount>
                    {showsDucats && (
                      <Amount currency="ducats">
                        {num(pageTotals.ducats)}
                      </Amount>
                    )}
                  </span>
                }
              />
              <Figure
                label="Account"
                title={`Every tab but Sets, which holds the same parts. ${LIQUIDATION_NOTE}`}
                value={<Amount currency="platinum">{num(accountValue)}</Amount>}
              />
            </div>
          </div>

          <TabsContent value={tab}>
            <InventoryTabView
              key={tab}
              tab={tab}
              entryCount={entries.length}
              visible={visible}
              unpriced={pageTotals.unpriced}
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
