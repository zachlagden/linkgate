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

export function GearIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <circle cx="8" cy="8" r="2" />
      <path d="M8 1.75v1.5M8 12.75v1.5M14.25 8h-1.5M3.25 8h-1.5M12.42 3.58l-1.06 1.06M4.64 11.36l-1.06 1.06M12.42 12.42l-1.06-1.06M4.64 4.64 3.58 3.58" />
    </Base>
  );
}

export function BackIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M9.5 3.5 5 8l4.5 4.5" />
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

export function ShieldIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M8 1.75 2.75 3.75v4c0 3.1 2.2 5.45 5.25 6.5 3.05-1.05 5.25-3.4 5.25-6.5v-4L8 1.75Z" />
      <path d="M8 5.25v3M8 10.6v.15" />
    </Base>
  );
}

export function GlobeIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <circle cx="8" cy="8" r="6.25" />
      <path d="M1.75 8h12.5M8 1.75c1.7 1.8 2.5 3.9 2.5 6.25S9.7 12.45 8 14.25C6.3 12.45 5.5 10.35 5.5 8S6.3 3.55 8 1.75Z" />
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

export function GripIcon(props: IconProps): ReactElement {
  return (
    <Base fill="currentColor" stroke="none" {...props}>
      <circle cx="6" cy="4" r="1.1" />
      <circle cx="10" cy="4" r="1.1" />
      <circle cx="6" cy="8" r="1.1" />
      <circle cx="10" cy="8" r="1.1" />
      <circle cx="6" cy="12" r="1.1" />
      <circle cx="10" cy="12" r="1.1" />
    </Base>
  );
}
