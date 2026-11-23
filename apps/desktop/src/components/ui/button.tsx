import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-45 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent)]",
  {
    variants: {
      variant: {
        default: "bg-[var(--accent)] text-white hover:brightness-110",
        secondary: "bg-[var(--surface-2)] text-[var(--text)] hover:bg-[var(--hover)]",
        outline: "border border-[var(--border)] bg-transparent text-[var(--text)] hover:bg-[var(--hover)]",
        ghost: "text-[var(--muted)] hover:bg-[var(--hover)] hover:text-[var(--text)]",
        danger: "bg-[var(--danger)] text-[var(--background)]",
      },
      size: { default: "h-10 px-4 py-2", sm: "h-8 px-3 text-xs", icon: "h-9 w-9" },
    },
    defaultVariants: { variant: "default", size: "default" },
  },
);
type Props = React.ComponentProps<"button"> & VariantProps<typeof buttonVariants> & {
  asChild?: boolean;
};
function Button({ className, variant, size, asChild = false, ...props }: Props) {
  const Comp = asChild ? Slot : "button";
  return (
    <Comp
      data-slot="button"
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  );
}
export { Button, buttonVariants };
