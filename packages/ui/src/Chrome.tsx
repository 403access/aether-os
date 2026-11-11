import type { ReactNode } from "react";

export type BadgeTone = "same" | "moved" | "partial" | "missing" | "local" | "info" | "muted";

export function Badge({
  tone = "muted",
  children,
  dot = true,
}: {
  tone?: BadgeTone;
  children: ReactNode;
  dot?: boolean;
}): ReactNode {
  return (
    <span className={`ds-badge ds-badge--${tone} aether-badge`}>
      {dot ? <span className="ds-dot" aria-hidden="true" /> : null}
      {children}
    </span>
  );
}

export function Kbd({ children }: { children: ReactNode }): ReactNode {
  return <kbd className="ds-kbd">{children}</kbd>;
}

export interface MeterProps {
  label: string;
  value: number;
  max?: number;
  detail?: string;
  tone?: "accent" | "green" | "amber" | "red";
}

export function Meter({ label, value, max = 100, detail, tone = "accent" }: MeterProps): ReactNode {
  const ratio = max <= 0 ? 0 : Math.max(0, Math.min(1, value / max));
  return (
    <div className="aether-meter">
      <div className="aether-meter__head">
        <span className="aether-meter__label">{label}</span>
        <span className="aether-meter__value">{detail ?? `${Math.round(ratio * 100)}%`}</span>
      </div>
      <div
        className="aether-meter__track"
        role="meter"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={max}
        aria-valuenow={value}
      >
        <span
          className={`aether-meter__fill aether-meter__fill--${tone}`}
          style={{ width: `${ratio * 100}%` }}
        />
      </div>
    </div>
  );
}

export interface SystemBarProps {
  session: string;
  agents: { running: number; max: number };
  cpuPercent: number;
  memoryMiB: number;
  audio: string;
  latencyMs?: number;
  right?: ReactNode;
}

/** Bottom chrome: session, agent budget, audio route and measured latency. */
export function SystemBar({
  session,
  agents,
  cpuPercent,
  memoryMiB,
  audio,
  latencyMs,
  right,
}: SystemBarProps): ReactNode {
  return (
    <footer className="aether-systembar ds-scope" aria-label="System status">
      <div className="aether-systembar__group">
        <span className="aether-systembar__session">
          <span className="aether-systembar__dot" aria-hidden="true" />
          {session}
        </span>
        <Badge tone={agents.running >= agents.max ? "partial" : "same"}>
          {agents.running}/{agents.max} agents
        </Badge>
        <Badge tone={cpuPercent > 85 ? "missing" : "info"} dot={false}>
          {cpuPercent.toFixed(0)}% CPU
        </Badge>
        <Badge tone="muted" dot={false}>
          {(memoryMiB / 1024).toFixed(1)} GiB
        </Badge>
        <Badge tone="local" dot={false}>
          ♪ {audio}
        </Badge>
        {latencyMs === undefined ? null : (
          <Badge tone={latencyMs > 30 ? "partial" : "same"} dot={false}>
            {latencyMs.toFixed(1)} ms frame
          </Badge>
        )}
      </div>
      <div className="aether-systembar__group">{right}</div>
    </footer>
  );
}
