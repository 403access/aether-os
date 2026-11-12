import { KABYLAKE_7700K, type AgentSpec } from "@aether/protocol";
import { planIsolation } from "@aether/agents";
import { Badge } from "@aether/ui";
import { LINKS } from "../data";

const LAYERS = [
  {
    name: "Shell",
    detail: "aether-shell · Rust · Smithay + wgpu",
    body: "One process owns the display: compositor, window layout, the command palette and system chrome. Rendering is a damage-tracked scene graph on Vulkan; there is no Qt, GTK or Electron in the idle path.",
    chips: ["120 Hz pacing", "Vulkan / DMA-BUF", "single draw pass"],
    metric: "~30–50 MB",
  },
  {
    name: "Applications",
    detail: "palette · chat · files · media · browser",
    body: "Thin native clients, not a desktop suite. The browser is a WebKitGTK shell; media wraps libmpv; files is the two-pane explorer below. Each app is a surface the shell can snapshot and sandbox independently.",
    chips: ["WebKitGTK", "libmpv", "shared UI kit"],
    metric: "on demand",
  },
  {
    name: "Daemons",
    detail: "aetherd · aether-index · aether-ipc",
    body: "aetherd is the broker: it plans isolation, admits agents against a 4c/8t budget, issues capabilities and records an audit chain. aether-index answers BM25 + fuzzy queries from memory. aether-ipc carries state over SPSC rings.",
    chips: ["capability broker", "BM25F index", "memfd rings"],
    metric: "~15–25 MB",
  },
  {
    name: "Platform",
    detail: "PipeWire · WirePlumber · XFS · NixOS",
    body: "PipeWire handles every audio route, in and out, including Bluetooth codecs. XFS on NVMe avoids copy-on-write write amplification. NixOS keeps the base declarative and reproducible while the kernel is compiled for this exact CPU.",
    chips: ["-march=native", "localmodconfig", "noatime"],
    metric: "~85–120 MB",
  },
];

const SAMPLES: Array<{ label: string; tone: "same" | "partial" | "missing"; spec: AgentSpec }> = [
  {
    label: "Trusted",
    tone: "same",
    spec: {
      id: "shell-overlay",
      name: "Shell overlay",
      trust: "trusted",
      tier: "in-process",
      cpuWeight: 10,
      memoryMiB: 64,
      capabilities: [],
      timeoutMs: 5_000,
    },
  },
  {
    label: "Untrusted",
    tone: "partial",
    spec: {
      id: "refactor-worker",
      name: "Refactor worker",
      trust: "untrusted",
      tier: "in-process",
      cpuWeight: 40,
      memoryMiB: 512,
      capabilities: [{ scope: "fs.read", resource: "fs:/work/aether" }],
      timeoutMs: 120_000,
    },
  },
  {
    label: "Hostile",
    tone: "missing",
    spec: {
      id: "test-runner",
      name: "Test runner",
      trust: "hostile",
      tier: "microvm",
      cpuWeight: 80,
      memoryMiB: 1_024,
      capabilities: [{ scope: "exec", resource: "vm:/" }],
      image: "aether/test-runner:0.1",
      timeoutMs: 300_000,
    },
  },
];

export function Architecture() {
  return (
    <section className="section" id="architecture">
      <div className="section__head">
        <p className="eyebrow">Architecture</p>
        <h2>Four layers. No abstraction for its own sake.</h2>
        <p className="section__lede">
          Each layer exists because removing it made the machine slower or less
          predictable. The full specification, failure modes and non-goals live in the
          docs.
        </p>
      </div>

      <div className="layers">
        {LAYERS.map((layer, index) => (
          <article className="layer-card" key={layer.name}>
            <span className="layer-card__index">{String(index + 1).padStart(2, "0")}</span>
            <div className="layer-card__body">
              <h3>{layer.name}</h3>
              <p className="layer-card__detail">{layer.detail}</p>
              <p>{layer.body}</p>
              <ul className="tag-list">
                {layer.chips.map((chip) => (
                  <li key={chip}>{chip}</li>
                ))}
              </ul>
            </div>
            <span className="layer-card__metric">{layer.metric}</span>
          </article>
        ))}
      </div>

      <div className="grid grid--2">
        <article className="panel">
          <h3>What we deliberately refused</h3>
          <ul className="check-list">
            <li>No GNOME, KDE, Qt or GTK runtime in the shell path.</li>
            <li>No X11 compatibility layer; Wayland only.</li>
            <li>No Electron for the palette, chat, files or media UI.</li>
            <li>No ZFS/Btrfs on the root volume; no background indexer polling.</li>
            <li>No cloud dependency for the AI surface; DeepSeek is reached over a local socket.</li>
          </ul>
        </article>
        <article className="panel">
          <h3>What we kept</h3>
          <ul className="check-list check-list--keep">
            <li>Linux, because drivers and the browser still need it.</li>
            <li>NixOS, because reproducibility is a performance feature.</li>
            <li>PipeWire, because it is the only sane audio graph on Linux.</li>
            <li>WebKitGTK, because writing a rendering engine is not the project.</li>
            <li>systemd, because seat and device management are not the project either.</li>
          </ul>
        </article>
      </div>

      <p className="section__footnote">
        Layer memory figures are <strong>budgets</strong>, not measured results. The
        measurement method and current numbers are tracked in{" "}
        <a href={`${LINKS.docs}reference/claims-and-evidence/`}>claims &amp; evidence</a>.
      </p>
    </section>
  );
}

export function Isolation() {
  return (
    <section className="section" id="isolation">
      <div className="section__head">
        <p className="eyebrow">Agent isolation</p>
        <h2>No PKU. Still no shared address space.</h2>
        <p className="section__lede">
          Kaby Lake predates Intel Memory Protection Keys, so the 10-nanosecond
          WRPKRU sandbox from the original design is unavailable. Aether encodes that
          fact as policy instead of pretending otherwise.
        </p>
      </div>

      <div className="grid grid--3">
        {SAMPLES.map((sample) => {
          const plan = planIsolation(sample.spec);
          return (
            <article className="isolation-card" key={sample.label}>
              <header>
                <Badge tone={sample.tone}>{sample.label}</Badge>
                <span className="isolation-card__id">{plan.agentId}</span>
              </header>
              <p className="isolation-card__tier">{plan.tier}</p>
              <p className="isolation-card__enforcement">
                enforcement <code>{plan.enforcement}</code>
              </p>
              <p className="isolation-card__rationale">{plan.rationale[0]}</p>
              {plan.warnings[0] === undefined ? (
                <p className="isolation-card__ok">No warnings for this plan.</p>
              ) : (
                <p className="isolation-card__warning">{plan.warnings[0]}</p>
              )}
              {plan.blocked ? (
                <p className="isolation-card__blocked">blocked: {plan.blockers[0]}</p>
              ) : null}
            </article>
          );
        })}
      </div>

      <div className="callout">
        <div>
          <h3>What replaces WRPKRU</h3>
          <p>
            Untrusted code is never placed in the shell&rsquo;s address space. It runs in
            a separate process with a Landlock filesystem allowlist, a seccomp-BPF
            syscall allowlist, <code>no_new_privs</code> and a per-agent UID. Memory
            separation comes from page tables, which cost a context switch but no
            correctness.
          </p>
        </div>
        <dl className="callout__facts">
          <div>
            <dt>PKU</dt>
            <dd>unavailable</dd>
          </div>
          <div>
            <dt>Landlock</dt>
            <dd>kernel 5.13+</dd>
          </div>
          <div>
            <dt>MicroVM</dt>
            <dd>VT-x + VT-d</dd>
          </div>
          <div>
            <dt>Concurrency</dt>
            <dd>6 agents max</dd>
          </div>
        </dl>
      </div>

      <div className="grid grid--2">
        <article className="panel">
          <h3>Collaboration is opt-in</h3>
          <p>
            Agents exchange state only through a channel opened by the broker. Each
            side receives a capability scoped to <code>channel:&lt;id&gt;</code>.
            Closing the channel revokes both handles, so a stale token fails in one
            map lookup.
          </p>
          <ul className="tag-list">
            <li>SPSC ring</li>
            <li>delegation narrows</li>
            <li>cascading revocation</li>
            <li>hash-chained audit</li>
          </ul>
        </article>
        <article className="panel">
          <h3>What a hostile agent cannot do</h3>
          <ul className="check-list">
            <li>Read another agent&rsquo;s heap or workspace.</li>
            <li>Open a socket without the <code>net</code> capability.</li>
            <li>Spawn another agent (no self-replication).</li>
            <li>Outlive its timeout or memory cap.</li>
            <li>Erase its audit trail without breaking the chain.</li>
          </ul>
        </article>
      </div>

      <p className="section__footnote">
        The cards above are produced at build time by the same{" "}
        <code>planIsolation()</code> function the supervisor uses, against the
        published {KABYLAKE_7700K.cpu} profile.{" "}
        <a href={`${LINKS.docs}architecture/agent-isolation/`}>Read the isolation spec →</a>
      </p>
    </section>
  );
}
