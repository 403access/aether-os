import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { withBase } from "../data";

interface Shot {
  id: string;
  file: string;
  title: string;
  shortcut: string;
  caption: string;
  alt: string;
  wide?: boolean;
}

const SHOTS: Shot[] = [
  {
    id: "palette",
    file: "screenshots/palette.png",
    title: "Command palette",
    shortcut: "Super+Space",
    caption:
      "Workspaces, windows, apps, files and commands in one ranked list. Prefixes narrow the mode; the Rust index answers in under two milliseconds.",
    alt: "Aether command palette open over a terminal, listing workspaces and terminal windows.",
    wide: true,
  },
  {
    id: "explorer",
    file: "screenshots/explorer.png",
    title: "Indexed explorer",
    shortcut: "Super+E",
    caption:
      "Local and archive roots side by side, 30-pixel rows and a status class per entry. Search runs against the index, not a directory walk.",
    alt: "Aether explorer listing indexed files with name, type and location columns.",
  },
  {
    id: "chat",
    file: "screenshots/chat.png",
    title: "Streaming chat",
    shortcut: "Super+C",
    caption:
      "A chat drawer that renders markdown tokens as they arrive — headings, lists and code — bounded per frame rather than per token.",
    alt: "Aether chat drawer showing streaming markdown with a code block and message input.",
  },
  {
    id: "audio",
    file: "screenshots/audio.png",
    title: "Audio routes",
    shortcut: "Super+A",
    caption:
      "Every output, input and latency preset is a first-class object. Switch routes with the keyboard and watch the input meter move.",
    alt: "Aether audio overlay listing analog, USB and Bluetooth outputs and inputs with an input level meter.",
  },
  {
    id: "shortcuts",
    file: "screenshots/shortcuts.png",
    title: "Keymap cheatsheet",
    shortcut: "Super+/",
    caption:
      "One central keymap, two profiles: Simple on Ctrl+Alt for VNC clients that swallow Super, Standard on Super for the machine itself.",
    alt: "Aether keyboard shortcut cheatsheet grouped into surfaces, windows, workspaces and overlays.",
  },
];

export function Screenshots() {
  const [active, setActive] = useState<Shot | null>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (active === null) return;
    closeRef.current?.focus();

    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setActive(null);
    };
    document.addEventListener("keydown", onKey);

    const previous = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = previous;
      triggerRef.current?.focus();
    };
  }, [active]);

  return (
    <section className="section screenshots" id="screenshots">
      <div className="section__head">
        <p className="eyebrow">Screenshots</p>
        <h2>The shell, not a render.</h2>
        <p className="section__lede">
          Every image below is a capture from the Aether compositor running the same
          Water tokens this page is built on. Select one to see it full size.
        </p>
      </div>

      <div className="shot-grid">
        {SHOTS.map((shot) => (
          <figure className={`shot${shot.wide === true ? " shot--wide" : ""}`} key={shot.id}>
            <button
              type="button"
              className="shot__button"
              aria-label={`Open screenshot: ${shot.title}`}
              onClick={(event) => {
                triggerRef.current = event.currentTarget;
                setActive(shot);
              }}
            >
              <span className="shot__bar">
                <span className="traffic" aria-hidden="true">
                  <i />
                  <i />
                  <i />
                </span>
                <span className="shot__name">aether-shell</span>
                <span className="shot__shortcut">{shot.shortcut}</span>
              </span>
              <img
                className="shot__image"
                src={withBase(shot.file)}
                alt={shot.alt}
                width={1280}
                height={800}
                loading="lazy"
              />
            </button>
            <figcaption className="shot__caption">
              <strong>{shot.title}</strong>
              <span>{shot.caption}</span>
            </figcaption>
          </figure>
        ))}
      </div>

      <p className="section__footnote">
        Captured from the host-side Smithay adapter at 1280×800. The same shell runs on
        the development VM and, at the P2 gate, on the reference machine.{" "}
        <a href={`${withBase("docs/")}architecture/smithay-backend/`}>How the shell is built →</a>
      </p>

      {active !== null &&
        createPortal(
          <div
            className="lightbox"
            role="dialog"
            aria-modal="true"
            aria-label={active.title}
            onClick={() => setActive(null)}
          >
            <div className="lightbox__panel" onClick={(event) => event.stopPropagation()}>
              <div className="lightbox__bar">
                <span>
                  <strong>{active.title}</strong> · {active.shortcut}
                </span>
                <button
                  ref={closeRef}
                  type="button"
                  className="ds-btn ds-btn--small ds-btn--ghost"
                  onClick={() => setActive(null)}
                >
                  Close ×
                </button>
              </div>
              <img
                src={withBase(active.file)}
                alt={active.alt}
                width={1280}
                height={800}
              />
            </div>
          </div>,
          document.body,
        )}
    </section>
  );
}
