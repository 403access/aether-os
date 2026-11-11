/* Water design system — showcase interactions. */

import { WaterSurface } from "./water.js";

const root = document.documentElement;
const TONE_LABELS = { lagoon: "Lagoon", ocean: "Ocean", deep: "Deep" };
const STORAGE = "dcc-design-prefs";

const state = {
  tone: root.dataset.waterTone || "ocean",
  background: root.dataset.waterBackground || "sea",
  effects: root.dataset.waterEffects !== "off",
  theme: root.dataset.theme === "light" ? "light" : "dark",
};

function readPrefs() {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE) || "null");
    if (!saved || typeof saved !== "object") return;
    if (TONE_LABELS[saved.tone]) state.tone = saved.tone;
    if (saved.background === "still" || saved.background === "sea") state.background = saved.background;
    if (typeof saved.effects === "boolean") state.effects = saved.effects;
    if (saved.theme === "light" || saved.theme === "dark") state.theme = saved.theme;
  } catch {
    /* ignore */
  }
}

function savePrefs() {
  try {
    localStorage.setItem(STORAGE, JSON.stringify(state));
  } catch {
    /* ignore */
  }
}

function apply() {
  root.dataset.waterTone = state.tone;
  root.dataset.waterBackground = state.background;
  root.dataset.waterEffects = state.effects ? "on" : "off";
  root.dataset.theme = state.theme;
  document
    .querySelectorAll("[data-tone]")
    .forEach((button) => {
      button.setAttribute("aria-pressed", String(button.dataset.tone === state.tone));
    });
  document.querySelectorAll("[data-tone-label]").forEach((node) => {
    node.textContent = TONE_LABELS[state.tone];
  });
}

readPrefs();
apply();

const surfaces = [];

const globalCanvas = document.getElementById("water-fx");
const globalSurface = globalCanvas
  ? new WaterSurface(globalCanvas, {
      tone: state.tone,
      speed: "flowing",
      intensity: "balanced",
      effects: state.effects,
      drips: true,
      clickRipples: true,
      caustics: true,
      enabled: state.effects,
    })
  : null;
if (globalSurface) surfaces.push(globalSurface);

const pond = document.getElementById("ripple-pond");
const pondCanvas = document.getElementById("pond-fx");
if (pond && pondCanvas) {
  surfaces.push(
    new WaterSurface(pondCanvas, {
      scope: pond,
      tone: state.tone,
      speed: "flowing",
      intensity: "lively",
      effects: true,
      drips: true,
      clickRipples: true,
      caustics: true,
      enabled: true,
    }),
  );
}

document.querySelectorAll("[data-tone]").forEach((button) => {
  button.addEventListener("click", () => {
    state.tone = button.dataset.tone || "ocean";
    apply();
    surfaces.forEach((surface) => surface.setTone(state.tone));
    savePrefs();
  });
});

const themeButton = document.querySelector("[data-theme-toggle]");
function syncThemeButton() {
  if (!themeButton) return;
  const light = state.theme === "light";
  themeButton.setAttribute("aria-pressed", String(light));
  const label = themeButton.querySelector("span");
  if (label) label.textContent = light ? "Dark" : "Light";
  const use = themeButton.querySelector("use");
  if (use) use.setAttribute("href", light ? "#i-sun" : "#i-moon");
}
syncThemeButton();

themeButton?.addEventListener("click", () => {
  state.theme = state.theme === "light" ? "dark" : "light";
  apply();
  syncThemeButton();
  savePrefs();
});

const backgroundButton = document.querySelector("[data-background-toggle]");
function syncBackgroundButton() {
  if (!backgroundButton) return;
  const sea = state.background === "sea";
  backgroundButton.setAttribute("aria-pressed", String(sea));
  backgroundButton.textContent = sea ? "Sea: on" : "Sea: off";
}
syncBackgroundButton();

backgroundButton?.addEventListener("click", () => {
  state.background = state.background === "sea" ? "still" : "sea";
  apply();
  syncBackgroundButton();
  savePrefs();
});

const effectsButton = document.querySelector("[data-effects-toggle]");
function syncEffectsButton() {
  if (!effectsButton) return;
  effectsButton.setAttribute("aria-pressed", String(state.effects));
  effectsButton.textContent = state.effects ? "Effects: on" : "Effects: off";
}
syncEffectsButton();

effectsButton?.addEventListener("click", () => {
  state.effects = !state.effects;
  apply();
  syncEffectsButton();
  // The pond stays interactive so the effect can still be inspected.
  globalSurface?.setEnabled(state.effects);
  savePrefs();
});

/* --------------------------------------------------------------- copy */

function fallbackCopy(text, done) {
  const area = document.createElement("textarea");
  area.value = text;
  area.setAttribute("readonly", "");
  area.style.position = "fixed";
  area.style.opacity = "0";
  document.body.appendChild(area);
  area.select();
  try {
    document.execCommand("copy");
    done();
  } catch {
    /* ignore */
  }
  document.body.removeChild(area);
}

document.querySelectorAll("[data-copy-target]").forEach((button) => {
  button.addEventListener("click", () => {
    const target = document.getElementById(button.dataset.copyTarget || "");
    if (!target) return;
    const text = target.innerText;
    const label = button.querySelector("span");
    const done = () => {
      if (!label) return;
      label.textContent = "Copied";
      window.setTimeout(() => {
        label.textContent = "Copy";
      }, 1600);
    };
    if (navigator.clipboard?.writeText) {
      navigator.clipboard.writeText(text).then(done).catch(() => fallbackCopy(text, done));
    } else {
      fallbackCopy(text, done);
    }
  });
});

document.querySelectorAll("[data-year]").forEach((node) => {
  node.textContent = String(new Date().getFullYear());
});
