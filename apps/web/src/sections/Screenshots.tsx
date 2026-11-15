import {
  useEffect,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { createPortal } from "react-dom";
import { LINKS, withBase } from "../data";

type Category = "overlays" | "windows" | "reference";
type FilterId = "all" | Category;

interface Shot {
  id: string;
  file: string;
  title: string;
  shortcut: string;
  category: Category;
  caption: string;
  alt: string;
  featured?: boolean;
}

const CATEGORY_LABEL: Record<Category, string> = {
  overlays: "Overlay",
  windows: "Window",
  reference: "Reference",
};

const CATEGORY_BADGE: Record<Category, string> = {
  overlays: "ds-badge ds-badge--local",
  windows: "ds-badge ds-badge--moved",
  reference: "ds-badge ds-badge--possible",
};

const FILTERS: Array<{ id: FilterId; label: string }> = [
  { id: "all", label: "All" },
  { id: "overlays", label: "Overlays" },
  { id: "windows", label: "Windows" },
  { id: "reference", label: "Reference" },
];

const SHOTS: Shot[] = [
  {
    id: "palette",
    file: "screenshots/palette.png",
    title: "Command palette",
    shortcut: "Super+Space",
    category: "overlays",
    featured: true,
    caption:
      "Workspaces, windows, apps, files and commands in one ranked list. Prefixes narrow the mode; the Rust index answers in under two milliseconds.",
    alt: "Aether command palette open over a terminal, listing workspaces and terminal windows.",
  },
  {
    id: "explorer",
    file: "screenshots/explorer.png",
    title: "Indexed explorer",
    shortcut: "Super+E",
    category: "windows",
    caption:
      "Local and archive roots side by side, 30-pixel rows and a status class per entry. Search runs against the index, not a directory walk.",
    alt: "Aether explorer listing indexed files with name, type and location columns.",
  },
  {
    id: "chat",
    file: "screenshots/chat.png",
    title: "Streaming chat",
    shortcut: "Super+C",
    category: "overlays",
    caption:
      "A chat drawer that renders markdown tokens as they arrive — headings, lists and code — bounded per frame rather than per token.",
    alt: "Aether chat drawer showing streaming markdown with a code block and message input.",
  },
  {
    id: "audio",
    file: "screenshots/audio.png",
    title: "Audio routes",
    shortcut: "Super+A",
    category: "overlays",
    caption:
      "Every output, input and latency preset is a first-class object. Switch routes with the keyboard and watch the input meter move.",
    alt: "Aether audio overlay listing analog, USB and Bluetooth outputs and inputs with an input level meter.",
  },
  {
    id: "remote",
    file: "screenshots/remote.png",
    title: "Remote display",
    shortcut: "Super+R",
    category: "windows",
    caption:
      "The same shell in a browser tab: WebRTC transport, WebCodecs decode and a virtual seat that accepts scancodes only. The pairing code is one-shot and the recording indicator is always visible.",
    alt: "Aether remote viewer in a browser showing the desktop, a live HUD and a recording indicator.",
  },
  {
    id: "shortcuts",
    file: "screenshots/shortcuts.png",
    title: "Keymap cheatsheet",
    shortcut: "Super+/",
    category: "reference",
    caption:
      "One central keymap, two profiles: Simple on Ctrl+Alt for VNC clients that swallow Super, Standard on Super for the machine itself.",
    alt: "Aether keyboard shortcut cheatsheet grouped into surfaces, windows, workspaces and overlays.",
  },
];

const FEATURED = SHOTS.find((shot) => shot.featured === true) ?? SHOTS[0]!;
const GALLERY = SHOTS.filter((shot) => shot.featured !== true);

function countFor(filter: FilterId): number {
  return filter === "all"
    ? GALLERY.length
    : GALLERY.filter((shot) => shot.category === filter).length;
}

function spotlight(event: ReactPointerEvent<HTMLButtonElement>): void {
  const frame = event.currentTarget.querySelector<HTMLElement>(".shot__frame");
  if (frame === null) return;
  const rect = frame.getBoundingClientRect();
  frame.style.setProperty("--spot-x", `${event.clientX - rect.left}px`);
  frame.style.setProperty("--spot-y", `${event.clientY - rect.top}px`);
}

export function Screenshots() {
  const [filter, setFilter] = useState<FilterId>("all");
  const [activeId, setActiveId] = useState<string | null>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const activeIdRef = useRef<string | null>(activeId);

  activeIdRef.current = activeId;
  const open = activeId !== null;
  const activeIndex =
    activeId === null ? -1 : SHOTS.findIndex((shot) => shot.id === activeId);
  const active = activeIndex === -1 ? null : SHOTS[activeIndex]!;

  const visible =
    filter === "all"
      ? GALLERY
      : GALLERY.filter((shot) => shot.category === filter);

  const openShot = (shot: Shot, trigger: HTMLButtonElement | null): void => {
    triggerRef.current = trigger;
    setActiveId(shot.id);
  };

  const step = (delta: number): void => {
    const current = activeIdRef.current;
    if (current === null) return;
    const index = SHOTS.findIndex((shot) => shot.id === current);
    if (index === -1) return;
    const next = (index + delta + SHOTS.length) % SHOTS.length;
    setActiveId(SHOTS[next]!.id);
  };

  useEffect(() => {
    if (!open) return;
    closeRef.current?.focus();

    const root = document.getElementById("root");
    root?.setAttribute("inert", "");

    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setActiveId(null);
      } else if (event.key === "ArrowRight") {
        event.preventDefault();
        step(1);
      } else if (event.key === "ArrowLeft") {
        event.preventDefault();
        step(-1);
      }
    };
    document.addEventListener("keydown", onKey);

    const previous = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = previous;
      root?.removeAttribute("inert");
      if (triggerRef.current?.isConnected === true) triggerRef.current.focus();
    };
  }, [open]);

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

      <div className="screenshots__lead">
        <figure className="shot shot--feature">
          <button
            type="button"
            className="shot__button"
            aria-label={`Open screenshot: ${FEATURED.title}`}
            onClick={(event) => openShot(FEATURED, event.currentTarget)}
            onPointerMove={spotlight}
            onPointerLeave={(event) => {
              const frame = event.currentTarget.querySelector<HTMLElement>(".shot__frame");
              frame?.style.removeProperty("--spot-x");
              frame?.style.removeProperty("--spot-y");
            }}
          >
            <span className="shot__bar">
              <span className="traffic" aria-hidden="true">
                <i />
                <i />
                <i />
              </span>
              <span className="shot__no">01</span>
              <span className="shot__name">aether-shell</span>
              <span className="shot__bar-right">
                <span className={CATEGORY_BADGE[FEATURED.category]}>
                  {CATEGORY_LABEL[FEATURED.category]}
                </span>
                <kbd className="ds-kbd">{FEATURED.shortcut}</kbd>
              </span>
            </span>
            <span className="shot__frame">
              <img
                className="shot__image"
                src={withBase(FEATURED.file)}
                alt={FEATURED.alt}
                width={1280}
                height={800}
                loading="lazy"
              />
              <span className="shot__open">
                Open full size <span aria-hidden="true">↗</span>
              </span>
            </span>
          </button>
        </figure>

        <aside className="screenshots__rail">
          <p className="eyebrow">Featured capture</p>
          <h3>{FEATURED.title}</h3>
          <p className="screenshots__rail-text">{FEATURED.caption}</p>

          <dl className="screenshots__facts">
            <div>
              <dt>Trigger</dt>
              <dd>{FEATURED.shortcut}</dd>
            </div>
            <div>
              <dt>Surface</dt>
              <dd>{CATEGORY_LABEL[FEATURED.category]}</dd>
            </div>
            <div>
              <dt>Capture</dt>
              <dd>1280 × 800</dd>
            </div>
            <div>
              <dt>Tokens</dt>
              <dd>Water · ocean</dd>
            </div>
          </dl>

          <div className="screenshots__rail-actions">
            <button
              type="button"
              className="ds-btn ds-btn--primary ds-btn--small"
              onClick={(event) => openShot(FEATURED, event.currentTarget)}
            >
              Open full size
            </button>
            <a
              className="ds-btn ds-btn--ghost ds-btn--small"
              href={`${LINKS.docs}architecture/smithay-backend/`}
            >
              How it is built ↗
            </a>
          </div>
        </aside>
      </div>

      <div className="screenshots__toolbar">
        <p className="screenshots__count" aria-live="polite">
          More captures <span>{String(countFor(filter)).padStart(2, "0")}</span>
        </p>
        <div className="ds-segmented" role="group" aria-label="Filter captures">
          {FILTERS.map((entry) => (
            <button
              key={entry.id}
              type="button"
              className="ds-segment"
              aria-pressed={filter === entry.id}
              onClick={() => setFilter(entry.id)}
            >
              {entry.label}
              <span className="screenshots__filter-count">{countFor(entry.id)}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="shot-grid">
        {visible.map((shot) => (
          <figure className="shot" key={shot.id}>
            <button
              type="button"
              className="shot__button"
              aria-label={`Open screenshot: ${shot.title}`}
              onClick={(event) => openShot(shot, event.currentTarget)}
              onPointerMove={spotlight}
              onPointerLeave={(event) => {
                const frame = event.currentTarget.querySelector<HTMLElement>(".shot__frame");
                frame?.style.removeProperty("--spot-x");
                frame?.style.removeProperty("--spot-y");
              }}
            >
              <span className="shot__bar">
                <span className="traffic" aria-hidden="true">
                  <i />
                  <i />
                  <i />
                </span>
                <span className="shot__no">
                  {String(SHOTS.indexOf(shot) + 1).padStart(2, "0")}
                </span>
                <span className="shot__name">aether-shell</span>
                <span className="shot__bar-right">
                  <span className={CATEGORY_BADGE[shot.category]}>
                    {CATEGORY_LABEL[shot.category]}
                  </span>
                  <kbd className="ds-kbd">{shot.shortcut}</kbd>
                </span>
              </span>
              <span className="shot__frame">
                <img
                  className="shot__image"
                  src={withBase(shot.file)}
                  alt={shot.alt}
                  width={1280}
                  height={800}
                  loading="lazy"
                />
                <span className="shot__open">
                  Open full size <span aria-hidden="true">↗</span>
                </span>
              </span>
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
            onClick={() => setActiveId(null)}
          >
            <div className="lightbox__panel" onClick={(event) => event.stopPropagation()}>
              <div className="lightbox__bar">
                <span className="lightbox__count">
                  {String(activeIndex + 1).padStart(2, "0")}
                  <span aria-hidden="true"> / </span>
                  {String(SHOTS.length).padStart(2, "0")}
                </span>
                <span className="lightbox__title">
                  <strong>{active.title}</strong>
                  <span className="lightbox__trigger">{active.shortcut}</span>
                </span>
                <div className="lightbox__actions">
                  <button
                    type="button"
                    className="ds-btn ds-btn--small ds-btn--ghost"
                    aria-label="Previous screenshot"
                    onClick={() => step(-1)}
                  >
                    ←
                  </button>
                  <button
                    type="button"
                    className="ds-btn ds-btn--small ds-btn--ghost"
                    aria-label="Next screenshot"
                    onClick={() => step(1)}
                  >
                    →
                  </button>
                  <button
                    ref={closeRef}
                    type="button"
                    className="ds-btn ds-btn--small ds-btn--ghost"
                    onClick={() => setActiveId(null)}
                  >
                    Close ×
                  </button>
                </div>
              </div>

              <div className="lightbox__stage">
                <img
                  key={active.id}
                  className="lightbox__image"
                  src={withBase(active.file)}
                  alt={active.alt}
                  width={1280}
                  height={800}
                />
              </div>

              <div className="lightbox__foot">
                <p className="lightbox__caption">{active.caption}</p>
                <div className="lightbox__thumbs" role="group" aria-label="All captures">
                  {SHOTS.map((shot) => (
                    <button
                      key={shot.id}
                      type="button"
                      className="lightbox__thumb"
                      aria-label={`View screenshot: ${shot.title}`}
                      aria-current={shot.id === active.id ? "true" : undefined}
                      onClick={() => setActiveId(shot.id)}
                    >
                      <img
                        src={withBase(shot.file)}
                        alt=""
                        aria-hidden="true"
                        width={1280}
                        height={800}
                      />
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>,
          document.body,
        )}
    </section>
  );
}
