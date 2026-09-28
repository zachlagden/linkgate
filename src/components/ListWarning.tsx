import type { ReactElement } from "react";
import type { ListHit } from "../lib/api";
import { joinWords, listLabel } from "../lib/format";
import { ShieldIcon } from "./Icons";

interface ListWarningProps {
  hits: ListHit[];
  host: string | null;
}

export function isSevere(hits: ListHit[]): boolean {
  return hits.some((hit) => hit.list !== "tracking");
}

export function ListWarning({ hits, host }: ListWarningProps): ReactElement {
  const severe = isSevere(hits);
  const names = joinWords(hits.map((hit) => listLabel(hit.list)));
  const parent = hits.find((hit) => hit.matched !== host)?.matched;
  return (
    <div
      role="alert"
      className={`flex gap-2.5 rounded-lg border px-3 py-2.5 ${
        severe ? "border-danger-line bg-danger-soft" : "border-line bg-well"
      }`}
    >
      <ShieldIcon className={`mt-px shrink-0 ${severe ? "text-danger" : "text-warn"}`} />
      <div className="min-w-0 text-[13px] leading-snug">
        <p className={`font-medium ${severe ? "text-danger" : "text-ink"}`}>
          On your {names} {hits.length > 1 ? "lists" : "list"}
        </p>
        <p className="mt-0.5 text-ink-2">
          {parent ? (
            <>
              Listed as <span className="font-mono text-[12px]">{parent}</span>.{" "}
            </>
          ) : null}
          You'll be asked to confirm before anything happens.
        </p>
      </div>
    </div>
  );
}
