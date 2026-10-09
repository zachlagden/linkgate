import type { ReactElement } from "react";
import type { Mode } from "../lib/api";
import { Button, Note } from "../components/ui";
import { StepList, type StepRowData } from "./Progress";

interface DoneProps {
  mode: Mode;
  version: string | null;
  steps: StepRowData[];
  fatal: string | null;
  canOpen: boolean;
  onOpen: () => void;
  onClose: () => void;
}

function headline(mode: Mode, version: string | null, failed: number, fatal: string | null): string {
  if (mode === "uninstall") return failed === 0 ? "linkgate is uninstalled" : "Uninstalled, with problems";
  if (fatal) return "linkgate wasn't installed";
  const name = version ? `linkgate ${version}` : "linkgate";
  if (failed > 0) return `${name} is installed, with problems`;
  return mode === "update" ? `${name} is up to date` : `${name} is installed`;
}

export function Done({ mode, version, steps, fatal, canOpen, onOpen, onClose }: DoneProps): ReactElement {
  const failed = steps.filter((step) => step.status === "failed").length;
  return (
    <>
      <div className="animate-enter flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-4 pt-5 pb-4">
        <h1 className="px-1 text-[20px] leading-tight font-semibold tracking-tight text-ink">
          {headline(mode, version, failed, fatal)}
        </h1>
        {fatal && <Note tone="danger">{fatal}</Note>}
        <StepList steps={steps} download={null} />
        {mode !== "uninstall" && !fatal && failed > 0 && (
          <Note>
            The steps that worked are in place. Fix the problems above and run the installer again to retry the rest.
          </Note>
        )}
      </div>
      <footer className="flex shrink-0 justify-end gap-2 border-t border-line bg-chrome px-4 py-3">
        <Button onClick={onClose} variant={canOpen ? "plain" : "primary"} autoFocus={!canOpen}>
          Close
        </Button>
        {canOpen && (
          <Button variant="primary" onClick={onOpen} autoFocus>
            Open linkgate settings
          </Button>
        )}
      </footer>
    </>
  );
}
