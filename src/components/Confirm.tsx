import { useEffect, useRef, type ReactElement } from "react";
import type { ListHit } from "../lib/api";
import { joinWords, listLabel } from "../lib/format";
import { Kbd } from "./Kbd";

interface ConfirmProps {
  actionLabel: string;
  question: string;
  hits: ListHit[];
  domain: string;
  busy: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function Confirm({ actionLabel, question, hits, domain, busy, onConfirm, onCancel }: ConfirmProps): ReactElement {
  const cancelRef = useRef<HTMLButtonElement>(null);
  useEffect(() => cancelRef.current?.focus(), []);
  const names = joinWords(hits.map((hit) => listLabel(hit.list)));

  return (
    <section className="animate-enter rounded-lg border border-danger-line px-4 pt-3.5 pb-3">
      <p className="text-[14px] font-medium text-ink">{question}</p>
      <p className="mt-1 text-[13px] leading-snug text-ink-2">
        <span className="font-medium text-ink">{domain}</span> is on your {names}{" "}
        {hits.length > 1 ? "lists" : "list"}. linkgate never blocks a link. It only checks that you meant it.
      </p>
      <div className="mt-3.5 flex justify-end gap-2">
        <button
          ref={cancelRef}
          type="button"
          onClick={onCancel}
          className="group flex h-8 items-center gap-2 rounded-md px-3 text-[13px] font-medium text-ink-2 outline-none transition-colors duration-150 hover:bg-hover hover:text-ink focus-visible:outline-2 focus-visible:outline-accent active:bg-press"
        >
          Go back <Kbd>Esc</Kbd>
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={onConfirm}
          className="flex h-8 items-center gap-2 rounded-md bg-danger px-3 text-[13px] font-medium text-bg outline-none transition-[filter,transform] duration-150 ease-out-quart hover:brightness-110 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-danger active:scale-[0.97] disabled:opacity-60"
        >
          {actionLabel}
        </button>
      </div>
    </section>
  );
}
