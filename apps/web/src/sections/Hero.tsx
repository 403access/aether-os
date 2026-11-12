import { LINKS } from "../data";

export function Hero() {
  return (
    <header className="hero">
      <div className="hero__copy">
        <p className="eyebrow">
          <span className="pulse" aria-hidden="true" /> Aether OS · v0.1 “Kestrel”
        </p>
        <h1>
          An operating
          <br />
          system, built for
          <br />
          <span className="accent">one user.</span>
        </h1>
        <p className="hero__lede">
          A Linux workstation built for exactly one machine: an i7-7700K, four cores,
          no portability tax. Rust compositor, zero-copy IPC, a 200 MB idle budget and
          agents that are isolated by the kernel, not by convention.
        </p>

        <div className="hero__actions">
          <a className="ds-btn ds-btn--primary" href={LINKS.architecture}>
            Read the architecture
          </a>
          <a className="ds-btn" href="#palette">
            Try the palette
          </a>
          <a className="ds-btn ds-btn--ghost" href={LINKS.github} target="_blank" rel="noreferrer">
            GitHub ↗
          </a>
        </div>

        <dl className="hero__stats">
          <div>
            <dt>Target</dt>
            <dd>i7-7700K</dd>
          </div>
          <div>
            <dt>Idle budget</dt>
            <dd>≤ 200 MB</dd>
          </div>
          <div>
            <dt>Frame budget</dt>
            <dd>8.3 ms</dd>
          </div>
          <div>
            <dt>Telemetry</dt>
            <dd>0 bytes</dd>
          </div>
        </dl>
      </div>

      <div className="hero__panel">
        <div className="shell-preview">
          <div className="shell-preview__bar">
            <span className="traffic" aria-hidden="true">
              <i />
              <i />
              <i />
            </span>
            <span className="shell-preview__name">aether-shell</span>
            <span className="shell-preview__metric">frame 7.2 / 8.3 ms</span>
          </div>

          <div className="shell-preview__scene">
            <div className="scene-window">
              <div className="scene-window__head">
                <span className="scene-dot" /> Chat · DeepSeek
                <span className="scene-window__tag">streaming</span>
              </div>
              <p className="scene-window__text">
                Landlock is available on this kernel. I&rsquo;ll run the refactor in a
                separate process with a read-only index and a 512 MiB cap.
              </p>
              <div className="scene-window__code">
                <span>agent.tier</span>
                <span className="accent">process-landlock</span>
                <span>enforcement</span>
                <span>seccomp + landlock</span>
              </div>
              <span className="scene-caret" aria-hidden="true" />
            </div>

            <div className="scene-palette">
              <div className="scene-palette__input">
                <span aria-hidden="true">⌘</span> index
                <kbd className="ds-kbd">esc</kbd>
              </div>
              <div className="scene-palette__row is-active">
                <span className="scene-palette__icon">▢</span>
                <span>
                  <strong>indexer.rs</strong>
                  <small>/work/aether/indexer/src</small>
                </span>
                <span className="ds-kbd">↵</span>
              </div>
              <div className="scene-palette__row">
                <span className="scene-palette__icon">⌘</span>
                <span>
                  <strong>Rebuild Search Index</strong>
                  <small>aether-index · FTS5</small>
                </span>
              </div>
              <div className="scene-palette__row">
                <span className="scene-palette__icon">◈</span>
                <span>
                  <strong>Refactor Worker</strong>
                  <small>untrusted · landlock</small>
                </span>
              </div>
            </div>

            <div className="scene-status">
              <span>
                <i className="scene-dot" /> aether@kestrel
              </span>
              <span>2/6 agents</span>
              <span>14% CPU</span>
              <span>♪ Speakers · Analog</span>
            </div>
          </div>
        </div>

        <p className="hero__caption">
          One compositor process owns the scene graph, the palette and system chrome.
          Surfaces are damage-tracked; the web view is just another texture.
        </p>
      </div>
    </header>
  );
}
