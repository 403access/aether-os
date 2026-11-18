import { chromium } from "@playwright/test";
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
await page.goto("https://403access.github.io/data-cloud-compare/design-system/os/", { waitUntil: "networkidle" });
await page.waitForTimeout(1500);
const data = await page.evaluate(() => {
  const pick = (sel) => {
    const el = document.querySelector(sel);
    if (!el) return null;
    const r = el.getBoundingClientRect();
    const s = getComputedStyle(el);
    return {
      rect: { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) },
      bg: s.backgroundColor, color: s.color, font: s.fontSize, weight: s.fontWeight,
      radius: s.borderRadius, border: s.border, shadow: s.boxShadow.slice(0, 120),
      padding: s.padding, gap: s.gap,
    };
  };
  const all = (sel) => [...document.querySelectorAll(sel)].map((el) => {
    const r = el.getBoundingClientRect();
    return { text: el.textContent.trim().slice(0, 40), x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) };
  });
  return {
    menubar: pick(".os-menubar"),
    logo: pick(".os-logo"),
    app: pick(".os-menubar__app"),
    menus: all(".os-menu-wrap > button, .os-menu-wrap > a"),
    rescaps: all(".os-resources"),
    status: all(".os-status"),
    clock: pick(".os-clock"),
    right: all(".os-menubar__group--right > *"),
    window: pick("#win-explorer"),
    titlebar: pick("#win-explorer .os-titlebar"),
    lights: all("#win-explorer .os-light"),
    title: pick("#win-explorer .os-titlebar__title"),
    dock: pick(".os-dock"),
    dockItems: all(".os-dock__item"),
    dockTiles: all(".os-dock__tile"),
    seps: all(".os-dock__sep"),
    explorerToolbar: pick("#win-explorer .osx-toolbar"),
    tokens: Object.fromEntries(["--ds-bg","--ds-bg-soft","--ds-surface","--ds-surface-2","--ds-surface-solid","--ds-popover","--ds-glass","--ds-line","--ds-line-strong","--ds-accent","--ds-text","--ds-text-2","--ds-text-3","--ds-green","--ds-bg-grad"].map((n) => [n, getComputedStyle(document.documentElement).getPropertyValue(n).trim()])),
  };
});
console.log(JSON.stringify(data, null, 2));
await browser.close();
