import { Field } from "@/components/ui/field";
import { Slider } from "@/components/ui/slider";
import { MAX_RIVEN_RANK } from "@/lib/rivens";

export function RivenRankSlider({
  id,
  label,
  rank,
  onRankChange,
}: {
  id: string;
  label: string;
  rank: number;
  onRankChange: (rank: number) => void;
}) {
  return (
    <Field id={id} label={label}>
      <Slider
        id={id}
        min={0}
        max={MAX_RIVEN_RANK}
        step={1}
        value={[rank]}
        onValueChange={([next = rank]) => onRankChange(next)}
      />
    </Field>
  );
}
