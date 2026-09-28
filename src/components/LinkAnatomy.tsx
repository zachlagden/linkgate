import { useLayoutEffect, useRef, useState, type ReactElement, type ReactNode } from "react";
import type { LinkView, Signal } from "../lib/api";

function QueryParts({ query }: { query: string }): ReactElement {
  const pairs = query.split("&");
  return (
    <>
      <span className="text-ink-3">?</span>
      {pairs.map((pair, index) => {
        const split = pair.indexOf("=");
        const key = split === -1 ? pair : pair.slice(0, split);
        const value = split === -1 ? null : pair.slice(split + 1);
        return (
          <span key={index}>
            {index > 0 && <span className="text-ink-3">&amp;</span>}
            <span className="text-seg-key">{key}</span>
            {value !== null && (
              <>
                <span className="text-ink-3">=</span>
                <span className="text-seg-value">{value}</span>
              </>
            )}
          </span>
        );
      })}
    </>
  );
}

function FullLink({ link }: { link: LinkView }): ReactElement {
  const ref = useRef<HTMLParagraphElement>(null);
  const [expanded, setExpanded] = useState(false);
  const [overflowing, setOverflowing] = useState(false);

  useLayoutEffect(() => {
    const element = ref.current;
    if (element && !expanded) setOverflowing(element.scrollHeight > element.clientHeight + 1);
  }, [link, expanded]);

  return (
    <div className="rounded-lg border border-line bg-well px-3 py-2.5">
      <p
        ref={ref}
        className={`font-mono text-[12px] leading-[1.65] break-all select-text ${expanded ? "max-h-56 overflow-y-auto" : "line-clamp-4"}`}
      >
        <span className={link.scheme === "http" ? "text-warn" : "text-ink-3"}>
          {link.scheme}
          {link.separator}
        </span>
        {link.userinfo && (
          <span className="rounded-sm bg-danger-soft text-danger">
            {link.userinfo}@
          </span>
        )}
        {link.subdomain && <span className="text-ink-2">{link.subdomain}.</span>}
        {link.domain && <span className="font-semibold text-ink">{link.domain}</span>}
        {link.port !== null && <span className="text-warn">:{link.port}</span>}
        <span className="text-seg-path">{link.path}</span>
        {link.query !== null && <QueryParts query={link.query} />}
        {link.fragment !== null && (
          <>
            <span className="text-ink-3">#</span>
            <span className="text-seg-hash">{link.fragment}</span>
          </>
        )}
      </p>
      {(overflowing || expanded) && (
        <button
          type="button"
          onClick={() => setExpanded((value) => !value)}
          className="mt-1.5 text-[12px] font-medium text-accent-ink outline-none hover:underline focus-visible:underline"
        >
          {expanded ? "Show less" : "Show full link"}
        </button>
      )}
    </div>
  );
}

const SIGNAL_TEXT: Record<Signal, (link: LinkView) => ReactNode> = {
  insecure: () => "Not encrypted",
  credentials: () => "Text before @ is not the site",
  punycode: (link) => (
    <>
      Looks like <span className="font-medium">{link.unicodeHost}</span>
    </>
  ),
  ip: () => "Raw IP address",
  port: (link) => `Port ${link.port}`,
};

const SIGNAL_TONE: Record<Signal, string> = {
  insecure: "text-warn",
  credentials: "text-danger",
  punycode: "text-danger",
  ip: "text-warn",
  port: "text-ink-2",
};

function Signals({ link }: { link: LinkView }): ReactElement | null {
  if (link.signals.length === 0) return null;
  return (
    <ul className="flex flex-wrap gap-x-3 gap-y-1">
      {link.signals.map((signal) => (
        <li key={signal} className={`flex items-center gap-1.5 text-[12px] ${SIGNAL_TONE[signal]}`}>
          <span className="h-1 w-1 rounded-full bg-current" />
          {SIGNAL_TEXT[signal](link)}
        </li>
      ))}
    </ul>
  );
}

export function LinkAnatomy({ link }: { link: LinkView }): ReactElement {
  return (
    <section className="flex flex-col gap-3">
      <h1 className="font-serif text-[23px] leading-[1.2] tracking-[-0.01em] break-all">
        {link.domain ? (
          <>
            {link.subdomain && <span className="text-ink-3">{link.subdomain}.</span>}
            <span className="font-semibold text-ink">{link.domain}</span>
          </>
        ) : (
          <span className="text-ink">{link.scheme === "file" ? "A file on this PC" : link.scheme}</span>
        )}
      </h1>
      <Signals link={link} />
      <FullLink link={link} />
    </section>
  );
}
