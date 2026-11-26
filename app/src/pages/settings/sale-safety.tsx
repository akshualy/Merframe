import { SelectField } from "@/components/option-select";
import { Section } from "@/components/page";
import type { FilterOption } from "@/lib/filters";
import type { SaleSafetyKeep, Settings } from "@/types";
import { type Patch, SwitchRow } from "./row";

const KEEP_OPTIONS: FilterOption<SaleSafetyKeep>[] = [
  { value: "unmastered", label: "Items you haven't built or mastered" },
  { value: "unbuilt", label: "Items you haven't built" },
  { value: "nothing", label: "Nothing" },
];

export function SaleSafetySetting({
  draft,
  patch,
}: {
  draft: Settings;
  patch: Patch;
}) {
  return (
    <Section
      title="Safe to List"
      description="Parts you still need for crafting don't count as safe to list on the Inventory and Market pages."
    >
      <div className="grid grid-col gap-4">
        <SelectField
          id="sale-safety-keep"
          label="Keep parts of"
          value={draft.sale_safety_keep}
          options={KEEP_OPTIONS}
          onChange={(value) => patch({ sale_safety_keep: value })}
          className="w-80"
        />
        <SwitchRow
          checked={draft.sale_safety_keep_favourites}
          onChange={(checked) =>
            patch({ sale_safety_keep_favourites: checked })
          }
        >
          Keep parts of favourited items
        </SwitchRow>
      </div>
    </Section>
  );
}
