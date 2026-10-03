import { CalendarIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Calendar } from "@/components/ui/calendar";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

const HOURS = Array.from({ length: 24 }, (_, hour) => hour);
const MINUTES = Array.from({ length: 60 }, (_, minute) => minute);

const pad = (value: number) => String(value).padStart(2, "0");

function TimePart({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: number;
  options: number[];
  onChange: (value: number) => void;
}) {
  return (
    <Select
      value={String(value)}
      onValueChange={(next) => onChange(Number(next))}
    >
      <SelectTrigger aria-label={label} className="w-20">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {options.map((option) => (
          <SelectItem key={option} value={String(option)}>
            {pad(option)}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

export function DateTimePicker({
  id,
  value,
  onChange,
}: {
  id?: string;
  value: Date;
  onChange: (value: Date) => void;
}) {
  const withTime = (hours: number, minutes: number) => {
    const next = new Date(value);
    next.setHours(hours, minutes, 0, 0);
    onChange(next);
  };

  return (
    <div className="flex flex-wrap items-center gap-2">
      <Popover>
        <PopoverTrigger asChild>
          <Button id={id} variant="outline" className="flex-1 justify-start">
            <CalendarIcon />
            {value.toLocaleDateString("en-GB", {
              day: "2-digit",
              month: "short",
              year: "numeric",
            })}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-auto p-0" align="start">
          <Calendar
            mode="single"
            captionLayout="dropdown"
            weekStartsOn={1}
            startMonth={new Date(2013, 0)}
            endMonth={new Date()}
            selected={value}
            defaultMonth={value}
            onSelect={(day) => {
              if (day) {
                const next = new Date(day);
                next.setHours(value.getHours(), value.getMinutes(), 0, 0);
                onChange(next);
              }
            }}
          />
        </PopoverContent>
      </Popover>
      <TimePart
        label="Hour"
        value={value.getHours()}
        options={HOURS}
        onChange={(hours) => withTime(hours, value.getMinutes())}
      />
      <span className="text-muted-foreground">:</span>
      <TimePart
        label="Minute"
        value={value.getMinutes()}
        options={MINUTES}
        onChange={(minutes) => withTime(value.getHours(), minutes)}
      />
    </div>
  );
}
