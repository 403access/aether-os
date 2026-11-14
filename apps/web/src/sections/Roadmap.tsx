import { Badge } from "@aether/ui";
import { LINKS } from "../data";

type PhaseStatus = "shipped" | "progress" | "next" | "planned";

const PHASES: Array<{
  id: string;
  title: string;
  status: PhaseStatus;
  summary: string;
  bullets: string[];
  gate: string;
}> = [
  {
    id: "P0",
    title: "Foundations",
    status: "shipped",
    summary: "This repository: the product surfaces, the tested core and the OS configuration.",
    bullets: [
      "Protocol, indexer, IPC ring, capability table, isolation planner, scheduler, palette and UI kit.",
      "Docusaurus documentation, marketing site, Water design system, unit + DOM + e2e tests.",
      "Design system and explorer imported from the Data Cloud Compare reference; FilePane rebuilt in React.",
    ],
    gate: "bun run verify + playwright + cargo test green in CI.",
  },
  {
    id: "P1",
    title: "NixOS bring-up",
    status: "progress",
    summary: "Boot the reference machine into a deterministic base with the custom kernel.",
    bullets: [
      "Flake pinned to nixpkgs; Kaby Lake hardware and kernel fragment; installer ISO and disko layout included.",
      "Install script, recovery path and a complete install guide for the reference machine.",
      "Separate aarch64 VM target boots the same base headless in ~10 s (test harness only).",
      "XFS root with noatime/largeio/inode64; ext4 scratch; tmpfs for the index.",
      "Remaining: first boot on the hardware, measured idle RAM and boot time.",
    ],
    gate: "Cold boot to a shell, < 1.5 s userspace, recorded by systemd-analyze.",
  },
  {
    id: "P2",
    title: "Compositor MVP",
    status: "progress",
    summary: "A Smithay shell that owns the scene graph, tiling and frame pacing.",
    bullets: [
      "Winit/EGL adapter tiles clients with a command palette, picker, draggable split, move/resize/snap/maximize and hover cursors.",
      "Next: app/file/agent palette providers, IME, fullscreen, multi-output snapping, DRM cursor themes, per-client damage, then udev/DRM.",
      "wgpu/Vulkan renderer, damage tracking, one draw pass for shell chrome.",
      "Frame scheduler from os/compositor wired to a real 120 Hz deadline.",
    ],
    gate: "Sustained 120 Hz with a 4K + 1080p output pair; no dropped frames at idle.",
  },
  {
    id: "P3",
    title: "Shell surfaces",
    status: "planned",
    summary: "The palette, chat and explorer become compositor layers, not app windows.",
    bullets: [
      "aether-index daemon with inotify updates over the shared-memory ring.",
      "Palette layer with the same ranking engine and provider adapters as the web demo.",
      "Explorer parity: keyboard navigation, status classes, two-root comparison.",
      "Design-system parity tests against the reference explorer at localhost:8787.",
    ],
    gate: "Palette p95 < 10 ms at 10k documents; explorer 10k rows at 120 Hz.",
  },
  {
    id: "P4",
    title: "Audio",
    status: "planned",
    summary: "Every input and output route is a first-class, scriptable object.",
    bullets: [
      "PipeWire + WirePlumber policy for analog, USB and Bluetooth devices.",
      "Device switch overlay and per-app routing.",
      "Latency measurement harness; no resampling in the default path.",
    ],
    gate: "Route switch < 100 ms; no xruns under a 64-frame buffer stress test.",
  },
  {
    id: "P5",
    title: "Agents & chat",
    status: "planned",
    summary: "DeepSeek chat and isolated workers under one supervisor.",
    bullets: [
      "aetherd broker: plans isolation, admits agents, issues and revokes capabilities.",
      "Streaming chat surface with markdown, code and tool-call traces.",
      "Landlock + seccomp profiles per worker; audit chain export.",
    ],
    gate: "Six concurrent agents with no cross-agent reads in a red-team suite.",
  },
  {
    id: "P6",
    title: "Applications",
    status: "planned",
    summary: "Browser, media and file management with no Electron or GTK chrome.",
    bullets: [
      "WebKitGTK shell with per-tab sandboxing and a minimal permission model.",
      "libmpv frontend with hardware decode and zero-copy frame handoff.",
      "File operations with copy/move progress on the status bar.",
    ],
    gate: "4K60 video at < 6% CPU; browser tab crash cannot take down the shell.",
  },
  {
    id: "P7",
    title: "Remote display",
    status: "planned",
    summary: "The desktop in a browser tab at interactive latency.",
    bullets: [
      "DMA-BUF capture → VA-API encode → str0m WebRTC → WebCodecs canvas.",
      "Low-latency encoding: CBR, no B-frames, intra-refresh.",
      "Input over data channels into a virtual seat.",
      "AF_XDP only if measurement shows the kernel stack is the bottleneck.",
    ],
    gate: "Glass-to-glass < 40 ms on LAN; no CPU pixel copies in the pipeline.",
  },
  {
    id: "P8",
    title: "Hardening",
    status: "planned",
    summary: "PGO, ThinLTO and microVM isolation, applied only where measurement pays.",
    bullets: [
      "Profile-guided optimization pipeline for shell, indexer and browser.",
      "Firecracker + VFIO for hostile workloads; snapshot/restore.",
      "Threat model review, capability fuzzing, kernel attack-surface audit.",
    ],
    gate: "10–25% throughput gain from PGO on recorded workloads, or the change is reverted.",
  },
];

const STATUS_LABEL: Record<PhaseStatus, string> = {
  shipped: "Shipped",
  progress: "In progress",
  next: "Next",
  planned: "Planned",
};

const STATUS_TONE = {
  shipped: "same",
  progress: "partial",
  next: "info",
  planned: "muted",
} as const;

export function Roadmap() {
  return (
    <section className="section" id="roadmap">
      <div className="section__head">
        <p className="eyebrow">Roadmap</p>
        <h2>Nine phases, each with an exit exam.</h2>
        <p className="section__lede">
          Ordering is driven by dependencies and risk: boot the machine, own the
          display, then make the surfaces fast. A phase is only done when its gate is
          measured, not when the code merges.
        </p>
      </div>

      <div className="roadmap">
        {PHASES.map((phase) => (
          <article className={`phase phase--${phase.status}`} key={phase.id}>
            <div className="phase__marker" aria-hidden="true">
              <span>{phase.id}</span>
            </div>
            <div className="phase__body">
              <header className="phase__head">
                <h3>{phase.title}</h3>
                <Badge tone={STATUS_TONE[phase.status]}>{STATUS_LABEL[phase.status]}</Badge>
              </header>
              <p className="phase__summary">{phase.summary}</p>
              <ul className="phase__list">
                {phase.bullets.map((bullet) => (
                  <li key={bullet}>{bullet}</li>
                ))}
              </ul>
              <p className="phase__gate">
                <strong>Gate</strong> {phase.gate}
              </p>
            </div>
          </article>
        ))}
      </div>

      <article className="source-card">
        <div className="source-card__icon" aria-hidden="true">
          ⌘
        </div>
        <div>
          <h3>Design system source of truth</h3>
          <p>
            P0 imports the Water design system and the explorer GUI from{" "}
            <code>/Users/olivermolnar/Developer/jottacloud-comparison</code>. The live
            reference instance runs at{" "}
            <a href="http://localhost:8787/" target="_blank" rel="noreferrer">
              http://localhost:8787/
            </a>{" "}
            and is the visual baseline for the shell and the two-pane explorer.
          </p>
          <ul className="phase__list">
            <li>Tokens and component primitives are vendored into <code>packages/design-system</code>.</li>
            <li>The explorer is rebuilt in React as <code>FilePane</code> in <code>packages/ui</code>.</li>
            <li>P3 adds parity tests against the reference before the shell adopts it.</li>
          </ul>
        </div>
      </article>

      <a className="text-link text-link--cta" href={LINKS.roadmap}>
        Full roadmap with risks and dependencies →
      </a>
    </section>
  );
}

const SWATCHES = [
  { name: "Deep sea", token: "--ds-bg" },
  { name: "Surface", token: "--ds-surface-solid" },
  { name: "Current", token: "--ds-accent" },
  { name: "Brand A", token: "--ds-brand-a" },
  { name: "Brand B", token: "--ds-brand-b" },
  { name: "Same", token: "--ds-green" },
  { name: "Partial", token: "--ds-amber" },
  { name: "Missing", token: "--ds-red" },
];

export function DesignSystem() {
  return (
    <section className="section" id="design">
      <div className="section__head">
        <p className="eyebrow">Design language</p>
        <h2>Water, borrowed on purpose.</h2>
        <p className="section__lede">
          The same tokens, components and motion rules power the OS shell, this site
          and the documentation. Dense typography, one cyan current, semantic status
          colours, and ambient motion you can switch off.
        </p>
      </div>

      <div className="design-grid">
        <article className="panel">
          <h3>Palette</h3>
          <div className="swatches">
            {SWATCHES.map((swatch) => (
              <div className="swatch" key={swatch.token}>
                <span className="swatch__chip" style={{ background: `var(${swatch.token})` }} />
                <span className="swatch__name">{swatch.name}</span>
                <code className="swatch__token">{swatch.token}</code>
              </div>
            ))}
          </div>
        </article>

        <article className="panel">
          <h3>Primitives</h3>
          <div className="primitive-row">
            <button type="button" className="ds-btn ds-btn--primary">
              Primary
            </button>
            <button type="button" className="ds-btn">
              Secondary
            </button>
            <button type="button" className="ds-btn ds-btn--ghost">
              Ghost
            </button>
          </div>
          <div className="primitive-row">
            <Badge tone="same">Same</Badge>
            <Badge tone="partial">Partial</Badge>
            <Badge tone="missing">Missing</Badge>
            <kbd className="ds-kbd">⌘K</kbd>
            <kbd className="ds-kbd">esc</kbd>
          </div>
          <p className="type-sample">
            <span className="type-sample__display">13 px dense UI</span>
            <span className="type-sample__body">
              Interface text is set at 13 px with a 1.6 line height; display copy uses
              tight tracking and a 1.15 leading.
            </span>
            <code className="type-sample__mono">--ds-font-mono · 11 px metadata</code>
          </p>
        </article>
      </div>

      <div className="design-credit">
        <p>
          Vendored from the Data Cloud Compare design system. The standalone component
          gallery is served at <code>/design-system/</code>.
        </p>
        <a className="ds-btn ds-btn--small" href={LINKS.designSystem}>
          Open the gallery ↗
        </a>
      </div>
    </section>
  );
}
