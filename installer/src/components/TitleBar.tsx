import type { ReactElement } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api } from "../lib/api";
import { CloseIcon, MinimizeIcon, Mark } from "./Icons";

const iconButton =
  "grid h-7 w-8 place-items-center rounded-md text-ink-3 outline-none transition-colors duration-150 hover:bg-hover hover:text-ink active:bg-press focus-visible:outline-2 focus-visible:outline-accent";

export function TitleBar({ title, canClose }: { title: string; canClose: boolean }): ReactElement {
  return (
    <header data-tauri-drag-region className="flex h-10 shrink-0 items-center gap-2 bg-chrome pr-1.5 pl-3">
      <Mark className="shrink-0" />
      <span data-tauri-drag-region className="flex-1 truncate text-[12.5px] font-medium text-ink-2">
        {title}
      </span>
      <button
        type="button"
        className={iconButton}
        onClick={() => void getCurrentWindow().minimize()}
        aria-label="Minimise"
      >
        <MinimizeIcon />
      </button>
      <button
        type="button"
        className={`${iconButton} hover:bg-danger hover:text-bg active:bg-danger disabled:opacity-40`}
        onClick={() => void api.dismiss()}
        disabled={!canClose}
        aria-label="Close"
      >
        <CloseIcon />
      </button>
    </header>
  );
}
