import { chromium } from "@playwright/test";
const out = "/private/var/folders/q1/drdd6kwj19z0l36drkth_y940000gn/T/opencode/ref";
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
await page.goto("https://403access.github.io/data-cloud-compare/design-system/os/", { waitUntil: "networkidle" });
await page.waitForTimeout(800);
await page.evaluate(() => {
  document.querySelectorAll(".os-window").forEach((el) => { el.hidden = true; el.classList.remove("is-open","is-focused"); });
  document.querySelector('[data-cmd="open-calendar"]').click();
});
await page.waitForTimeout(700);
await page.screenshot({ path: `${out}/calendar.png` });
console.log(await page.evaluate(() => {
  const pick = (sel) => {
    const el = document.querySelector(sel);
    if (!el) return null;
    const r = el.getBoundingClientRect(); const s = getComputedStyle(el);
    return { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height), font: s.fontSize, color: s.color, bg: s.backgroundColor, radius: s.borderRadius, padding: s.padding, gap: s.gap };
  };
  const all = (sel) => [...document.querySelectorAll(sel)].map((el) => { const r = el.getBoundingClientRect(); return { t: el.textContent.trim().slice(0,20), x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height), cls: el.className }; });
  return {
    toolbar: pick(".os-calendar-app__toolbar"),
    month: pick(".os-calendar-app__month h2"),
    weekdays: pick(".os-calendar-app__weekdays"),
    dayCells: all(".os-calendar-app__grid > *").slice(0, 10),
    agenda: pick(".os-calendar-app__agenda"),
    agendaHead: pick(".os-agenda__head"),
    events: all(".os-agenda__event"),
    next: all(".os-agenda__event b, .os-agenda__event small"),
    divider: pick(".os-calendar-app__agenda"),
    css: [...document.styleSheets].flatMap(s => { try { return [...s.cssRules].map(r => r.cssText) } catch { return [] } }).filter(t => t.includes("os-calendar-app") || t.includes("os-agenda")).join("\n").slice(0, 6000),
  };
}));
await browser.close();
