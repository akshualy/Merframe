import { SelectField } from "@/components/option-select";
import { Section } from "@/components/page";
import type { Motion, Settings, TextSize } from "@/types";
import type { Patch } from "./row";

const TEXT_SIZES: { value: TextSize; label: string }[] = [
  { value: "small", label: "Small" },
  { value: "medium", label: "Medium" },
  { value: "large", label: "Large" },
];

const MOTION: { value: Motion; label: string }[] = [
  { value: "system", label: "Follow the system" },
  { value: "reduced", label: "Reduced" },
  { value: "full", label: "Full" },
];

export function DisplaySetting({
  draft,
  patch,
}: {
  draft: Settings;
  patch: Patch;
}) {
  return (
    <Section
      title="Display"
      description="Applies to the app and its overlays. Charts keep their size. The theme follows the system until you pick one with the button in the top bar, which cycles dark, light and system."
    >
      <div className="grid gap-3 @sm:grid-cols-2">
        <SelectField
          id="text-size"
          label="Text size"
          value={draft.text_size}
          options={TEXT_SIZES}
          onChange={(value) => patch({ text_size: value })}
          className="w-full"
        />
        <SelectField
          id="motion"
          label="Animations"
          value={draft.motion}
          options={MOTION}
          onChange={(value) => patch({ motion: value })}
          className="w-full"
        />
      </div>
    </Section>
  );
}
