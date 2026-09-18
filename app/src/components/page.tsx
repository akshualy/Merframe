import { AlertCircle } from "lucide-react";
import type { ComponentProps, ReactNode } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Hint } from "@/components/ui/hint";
import { Skeleton } from "@/components/ui/skeleton";
import type { Quote } from "@/lib/quotes";
import { cn } from "@/lib/utils";

export function Quoted({ quote }: { quote: Quote }) {
  return (
    <span>
      {`"${quote.line}"`}{" "}
      <span className="text-muted-foreground/70">{quote.speaker}</span>
    </span>
  );
}

export function Page({
  title,
  description,
  actions,
  children,
}: {
  title: string;
  description: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-6 p-6">
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div className="flex flex-col gap-1">
          <h1 className="text-primary text-2xl font-bold">{title}</h1>
          <p className="text-muted-foreground text-sm">{description}</p>
        </div>
        {actions && <div className="flex items-center gap-2">{actions}</div>}
      </header>
      {children}
    </div>
  );
}

export function Section({
  title,
  description,
  action,
  className,
  children,
}: {
  title?: string;
  description?: ReactNode;
  action?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  return (
    <Card className={cn("gap-4", className)}>
      {(title || description || action) && (
        <CardHeader className="grid-cols-[1fr_auto] items-center">
          <div className="flex flex-col gap-1">
            {title && <CardTitle>{title}</CardTitle>}
            {description && (
              <p className="text-muted-foreground text-sm">{description}</p>
            )}
          </div>
          {action}
        </CardHeader>
      )}
      <CardContent>{children}</CardContent>
    </Card>
  );
}

export function Surface({ className, ...props }: ComponentProps<"div">) {
  return <Card className={cn("gap-2 border p-4", className)} {...props} />;
}

export function Stat({
  label,
  value,
  hint,
}: {
  label: string;
  value: ReactNode;
  hint?: ReactNode;
}) {
  return (
    <Surface className="gap-1 px-4 py-3">
      <Hint as="span">{label}</Hint>
      <span className="text-foreground text-xl font-bold">{value}</span>
      {hint && <Hint as="span">{hint}</Hint>}
    </Surface>
  );
}

export function CardGrid({
  className,
  children,
}: {
  className?: string;
  children: ReactNode;
}) {
  return (
    <div
      className={cn(
        "grid gap-3 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4",
        className,
      )}
    >
      {children}
    </div>
  );
}

export function TableSkeleton({ rows = 8 }: { rows?: number }) {
  const placeholders = Array.from({ length: rows }, (_, index) => ({
    id: `placeholder-${index}`,
    opacity: 1 - index * 0.07,
  }));
  return (
    <div className="flex flex-col gap-3">
      <Skeleton className="h-9 w-72" />
      <Surface>
        {placeholders.map((placeholder) => (
          <Skeleton
            key={placeholder.id}
            className="h-6 w-full"
            style={{ opacity: placeholder.opacity }}
          />
        ))}
      </Surface>
    </div>
  );
}

export function EmptyNote({ children }: { children: ReactNode }) {
  return <p className="text-muted-foreground text-sm">{children}</p>;
}

export function EmptyPanel({ children }: { children: ReactNode }) {
  return (
    <p className="text-muted-foreground rounded-xl border p-8 text-center text-sm">
      {children}
    </p>
  );
}

export function ErrorNote({ message }: { message: string }) {
  return (
    <div className="text-destructive flex items-center gap-2 text-sm">
      <AlertCircle className="size-4" />
      {message}
    </div>
  );
}
