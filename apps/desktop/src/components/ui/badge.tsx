import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";
export function Badge({ className, ...props }: ComponentProps<"span">) {
  return <span className={cn("inline-flex items-center gap-1 rounded-md border border-[var(--border)] px-2 py-0.5 text-[11px] font-semibold tracking-wide text-[var(--muted)]", className)} {...props} />;
}
