import type { ReactElement, ReactNode } from "react";

export function Kbd({ children }: { children: ReactNode }): ReactElement {
  return (
    <kbd className="inline-grid h-5 min-w-5 place-items-center rounded-[5px] border border-line px-1 font-mono text-[10.5px] leading-none text-ink-3 transition-colors duration-150 group-hover:text-ink-2">
      {children}
    </kbd>
  );
}
