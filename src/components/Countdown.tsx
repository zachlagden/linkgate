import type { ReactElement } from "react";

interface CountdownProps {
  runKey: number;
  durationMs: number;
  running: boolean;
  paused: boolean;
  onElapsed: () => void;
}

export function Countdown({ runKey, durationMs, running, paused, onElapsed }: CountdownProps): ReactElement {
  return (
    <div className="h-[2px] shrink-0 bg-line" role="presentation">
      {running && (
        <div
          key={runKey}
          className="h-full origin-left bg-accent"
          style={{
            animation: `countdown ${durationMs}ms linear forwards`,
            animationPlayState: paused ? "paused" : "running",
          }}
          onAnimationEnd={onElapsed}
        />
      )}
    </div>
  );
}
