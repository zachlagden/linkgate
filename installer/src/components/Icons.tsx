import type { ReactElement, SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

function Base({ children, ...props }: IconProps): ReactElement {
  return (
    <svg
      viewBox="0 0 16 16"
      width={16}
      height={16}
      fill="none"
      stroke="currentColor"
      strokeWidth={1.4}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...props}
    >
      {children}
    </svg>
  );
}

export function CloseIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M4 4l8 8M12 4l-8 8" />
    </Base>
  );
}

export function MinimizeIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M3.5 8h9" />
    </Base>
  );
}

export function CheckIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M3.25 8.5l3 3 6.5-7" />
    </Base>
  );
}

export function CrossIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M4.5 4.5l7 7M11.5 4.5l-7 7" />
    </Base>
  );
}

export function SpinnerIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M8 1.75a6.25 6.25 0 1 0 6.25 6.25" />
    </Base>
  );
}

export function CopyIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <rect x="5.25" y="5.25" width="8" height="8" rx="1.75" />
      <path d="M10.75 5.25V4a1.25 1.25 0 0 0-1.25-1.25H4A1.25 1.25 0 0 0 2.75 4v5.5A1.25 1.25 0 0 0 4 10.75h1.25" />
    </Base>
  );
}

export function Mark(props: IconProps): ReactElement {
  return (
    <svg viewBox="0 0 1024 1024" width={16} height={16} aria-hidden="true" {...props}>
      <rect width="1024" height="1024" rx="232" fill="var(--accent)" />
      <g fill="none" stroke="var(--bg)" strokeWidth={96} strokeLinecap="round" strokeLinejoin="round">
        <path d="M512 812V560" />
        <path d="M512 560C512 440 400 392 312 300" />
        <path d="M512 560C512 440 624 392 712 300" />
        <path d="M292 452V292H452" />
        <path d="M732 452V292H572" />
      </g>
    </svg>
  );
}
