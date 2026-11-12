import { useEffect } from "react";
import { WaterSurface } from "@aether/design-system/water.js";
import { LINKS } from "./data";
import { Hero } from "./sections/Hero";
import { PaletteDemo } from "./sections/PaletteDemo";
import { Architecture, Isolation } from "./sections/Architecture";
import { ExplorerDemo, Performance } from "./sections/Workspace";
import { DesignSystem, Roadmap } from "./sections/Roadmap";

export function App() {
  useEffect(() => {
    const canvas = document.getElementById("water-fx");
    if (!(canvas instanceof HTMLCanvasElement)) return;
    if (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true) return;

    const surface = new WaterSurface(canvas, {
      tone: "ocean",
      speed: "flowing",
      intensity: "subtle",
      effects: true,
      drips: true,
      clickRipples: true,
      caustics: true,
    });
    return () => surface.destroy();
  }, []);

  return (
    <div className="site" id="top">
      <div className="sea" aria-hidden="true">
        <svg className="sea-wave sea-wave--far" viewBox="0 0 2880 320" preserveAspectRatio="none">
          <path d="M0 150 C 240 96 480 204 720 150 C 960 96 1200 204 1440 150 C 1680 96 1920 204 2160 150 C 2400 96 2640 204 2880 150 L 2880 320 L 0 320 Z" />
        </svg>
        <svg className="sea-wave sea-wave--mid" viewBox="0 0 2880 320" preserveAspectRatio="none">
          <path d="M0 170 C 240 126 480 214 720 170 C 960 126 1200 214 1440 170 C 1680 126 1920 214 2160 170 C 2400 126 2640 214 2880 170 L 2880 320 L 0 320 Z" />
        </svg>
        <svg className="sea-wave sea-wave--near" viewBox="0 0 2880 320" preserveAspectRatio="none">
          <path d="M0 190 C 240 156 480 224 720 190 C 960 156 1200 224 1440 190 C 1680 156 1920 224 2160 190 C 2400 156 2640 224 2880 190 L 2880 320 L 0 320 Z" />
        </svg>
      </div>
      <canvas id="water-fx" className="water-fx" aria-hidden="true" />

      <nav className="site-nav" aria-label="Primary">
        <a className="brand" href="#top" aria-label="Aether OS home">
          <svg viewBox="0 0 32 32" aria-hidden="true" className="brand__mark">
            <defs>
              <linearGradient id="brand-gradient" x1="0" y1="0" x2="1" y2="1">
                <stop offset="0" stopColor="#22c8e8" />
                <stop offset="1" stopColor="#3f7cff" />
              </linearGradient>
            </defs>
            <path
              d="M2 17c4-5 8-5 12 0s8 5 12 0"
              fill="none"
              stroke="url(#brand-gradient)"
              strokeWidth="2.6"
              strokeLinecap="round"
            />
            <path
              d="M2 23c4-5 8-5 12 0s8 5 12 0"
              fill="none"
              stroke="url(#brand-gradient)"
              strokeWidth="1.8"
              strokeLinecap="round"
              opacity="0.55"
            />
            <circle cx="16" cy="9" r="2.2" fill="#38d6ff" />
          </svg>
          <span className="brand__name">
            Aether<span className="brand__os">OS</span>
          </span>
        </a>

        <div className="site-nav__links">
          <a href="#architecture">Architecture</a>
          <a href="#isolation">Isolation</a>
          <a href="#palette">Palette</a>
          <a href="#explorer">Explorer</a>
          <a href="#roadmap">Roadmap</a>
        </div>

        <div className="site-nav__actions">
          <a className="ds-btn ds-btn--ghost ds-btn--small" href={LINKS.github} target="_blank" rel="noreferrer">
            GitHub ↗
          </a>
          <a className="ds-btn ds-btn--primary ds-btn--small" href={LINKS.docs}>
            Docs
          </a>
        </div>
      </nav>

      <main className="site-main">
        <Hero />
        <PaletteDemo />
        <Architecture />
        <Isolation />
        <ExplorerDemo />
        <Performance />
        <Roadmap />
        <DesignSystem />

        <section className="closing">
          <p className="eyebrow">Start here</p>
          <h2>Build the machine you actually own.</h2>
          <p>
            The architecture, product requirements and per-subsystem specs are
            published in full — including the parts that are still targets, and the
            decisions we reversed.
          </p>
          <div className="closing__actions">
            <a className="ds-btn ds-btn--primary" href={LINKS.docs}>
              Read the documentation
            </a>
            <a className="ds-btn" href={LINKS.roadmap}>
              Roadmap &amp; gates
            </a>
            <a className="ds-btn ds-btn--ghost" href={LINKS.github} target="_blank" rel="noreferrer">
              Source ↗
            </a>
          </div>
        </section>
      </main>

      <footer className="site-footer">
        <div className="site-footer__brand">
          <span className="brand__name">
            Aether<span className="brand__os">OS</span>
          </span>
          <p>
            A single-target Linux workstation OS. Built with Bun, TypeScript, React,
            Docusaurus, Rust and Nix.
          </p>
          <p className="site-footer__fine">
            Performance figures are budgets or reference-host measurements, not
            guarantees for your machine. Aether targets exactly one CPU.
          </p>
        </div>
        <div className="site-footer__cols">
          <div>
            <h3>Product</h3>
            <a href="#architecture">Architecture</a>
            <a href="#palette">Palette</a>
            <a href="#explorer">Explorer</a>
            <a href="#performance">Performance ledger</a>
          </div>
          <div>
            <h3>Docs</h3>
            <a href={LINKS.docs}>Overview</a>
            <a href={`${LINKS.docs}product/prd`}>Product requirements</a>
            <a href={`${LINKS.docs}architecture/agent-isolation`}>Agent isolation</a>
            <a href={`${LINKS.docs}reference/claims-and-evidence`}>Claims &amp; evidence</a>
          </div>
          <div>
            <h3>Project</h3>
            <a href={LINKS.github} target="_blank" rel="noreferrer">
              GitHub
            </a>
            <a href={LINKS.designSystem}>Design system</a>
            <a href={`${LINKS.docs}contributing/development`}>Contributing</a>
            <a href={LINKS.roadmap}>Roadmap</a>
          </div>
        </div>
        <div className="site-footer__bottom">
          <span>MIT licensed · v0.1.0-alpha.0</span>
          <span>No telemetry. No portability layer. One machine.</span>
        </div>
      </footer>
    </div>
  );
}
