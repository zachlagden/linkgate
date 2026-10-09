import type { ReactElement } from "react";
import type { StepStatus } from "../lib/api";
import { CheckIcon, CrossIcon, SpinnerIcon } from "../components/Icons";

export interface StepRowData {
  id: string;
  label: string;
  status: StepStatus | "pending";
  message: string | null;
}

export interface DownloadProgress {
  done: number;
  total: number | null;
}

function megabytes(bytes: number): string {
  return (bytes / (1024 * 1024)).toFixed(1);
}

function StatusIcon({ status }: { status: StepRowData["status"] }): ReactElement {
  switch (status) {
    case "running":
      return <SpinnerIcon className="animate-spin-slow text-accent-ink" />;
    case "done":
      return <CheckIcon className="text-accent-ink" />;
    case "failed":
      return <CrossIcon className="text-danger" />;
    default:
      return <span className="h-2 w-2 rounded-full border border-line" />;
  }
}

export function StepList({ steps, download }: { steps: StepRowData[]; download: DownloadProgress | null }): ReactElement {
  return (
    <ol className="flex flex-col rounded-lg border border-line">
      {steps.map((step, index) => (
        <li key={step.id} className={`flex gap-3 px-3 py-2.5 ${index > 0 ? "border-t border-line" : ""}`}>
          <span className="mt-0.5 grid h-4 w-4 shrink-0 place-items-center">
            <StatusIcon status={step.status} />
          </span>
          <div className="min-w-0 flex-1">
            <p className={`text-[13.5px] ${step.status === "pending" ? "text-ink-3" : "font-medium text-ink"}`}>
              {step.label}
            </p>
            {step.message && (
              <p
                className={`select-text mt-0.5 text-[12.5px] leading-snug ${step.status === "failed" ? "text-danger" : "text-ink-3"}`}
              >
                {step.message}
              </p>
            )}
            {step.id === "download" && step.status === "running" && download && (
              <div className="mt-1.5 flex flex-col gap-1">
                <div className="h-1 overflow-hidden rounded-full bg-press">
                  <div
                    className="h-full rounded-full bg-accent transition-[width] duration-150"
                    style={{ width: `${download.total ? Math.min(100, (download.done / download.total) * 100) : 40}%` }}
                  />
                </div>
                <p className="font-mono text-[11px] text-ink-3">
                  {megabytes(download.done)}
                  {download.total ? ` of ${megabytes(download.total)}` : ""} MB
                </p>
              </div>
            )}
          </div>
        </li>
      ))}
    </ol>
  );
}

export function Progress({ title, steps, download }: { title: string; steps: StepRowData[]; download: DownloadProgress | null }): ReactElement {
  return (
    <div className="animate-enter flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-4 pt-5 pb-4">
      <h1 className="px-1 text-[20px] leading-tight font-semibold tracking-tight text-ink">{title}</h1>
      <StepList steps={steps} download={download} />
    </div>
  );
}
