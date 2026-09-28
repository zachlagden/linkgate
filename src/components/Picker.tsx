import { useCallback, useEffect, useState, type ReactElement } from "react";
import { api, errorMessage, type BrowserView, type InitialState } from "../lib/api";
import { ActionList } from "./ActionList";
import { Confirm } from "./Confirm";
import { LinkAnatomy } from "./LinkAnatomy";
import { ListWarning } from "./ListWarning";

type PendingAction = { kind: "open"; browser: BrowserView } | { kind: "copy" };

interface PickerProps {
  state: InitialState;
  browsers: BrowserView[];
  onSettings: () => void;
  onRestartCountdown: () => void;
}

export function Picker({ state, browsers, onSettings, onRestartCountdown }: PickerProps): ReactElement {
  const { link, hits, raw } = state;
  const [pending, setPending] = useState<PendingAction | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const flagged = hits.length > 0;
  const host = link?.subdomain ? `${link.subdomain}.${link.domain}` : (link?.domain ?? null);

  const run = useCallback(async (action: PendingAction): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      if (action.kind === "open") await api.openIn(action.browser.id);
      else await api.copyLink();
    } catch (failure) {
      setError(errorMessage(failure));
      setPending(null);
      setBusy(false);
    }
  }, []);

  const request = useCallback(
    (action: PendingAction): void => {
      if (busy) return;
      if (flagged) {
        setPending(action);
        onRestartCountdown();
        return;
      }
      void run(action);
    },
    [busy, flagged, onRestartCountdown, run],
  );

  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      if (event.ctrlKey || event.altKey || event.metaKey) return;
      if (pending) {
        if (event.key === "Escape") setPending(null);
        if (event.key === "Enter") void run(pending);
        return;
      }
      if (event.key === "Escape") void api.dismiss();
      else if (event.key.toLowerCase() === "c") request({ kind: "copy" });
      else if (/^[1-9]$/.test(event.key) && link?.openable) {
        const browser = browsers[Number(event.key) - 1];
        if (browser) request({ kind: "open", browser });
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [browsers, link, pending, request, run]);

  return (
    <div className="flex flex-col gap-4 px-4 pt-5 pb-3">
      <div className="flex flex-col gap-3 px-1">
        {link ? (
          <LinkAnatomy link={link} />
        ) : (
          <section className="flex flex-col gap-2">
            <h1 className="font-serif text-[21px] leading-tight text-ink">This isn't a web link</h1>
            <p className="rounded-lg border border-line bg-well px-3 py-2.5 font-mono text-[12px] break-all text-ink-2 select-text">
              {raw}
            </p>
          </section>
        )}
        {flagged && <ListWarning hits={hits} host={host} />}
        {link && !link.openable && (
          <p className="text-[13px] text-ink-2">Browsers can't open {link.scheme} links. You can still copy it.</p>
        )}
      </div>
      {pending ? (
        <Confirm
          question={pending.kind === "open" ? `Open in ${pending.browser.name}?` : "Copy this link?"}
          actionLabel={pending.kind === "open" ? "Open anyway" : "Copy anyway"}
          hits={hits}
          domain={host ?? "This site"}
          busy={busy}
          onConfirm={() => void run(pending)}
          onCancel={() => setPending(null)}
        />
      ) : (
        <ActionList
          browsers={browsers}
          canOpen={Boolean(link?.openable)}
          busy={busy}
          onOpen={(browser) => request({ kind: "open", browser })}
          onCopy={() => request({ kind: "copy" })}
          onSettings={onSettings}
        />
      )}
      {error && (
        <p role="alert" className="px-3 text-[13px] text-danger">
          {error}
        </p>
      )}
    </div>
  );
}
