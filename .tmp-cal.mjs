import { chromium } from "@playwright/test";
const out = "/private/var/folders/q1/drdd6kwj19z0l36drkth_y940000gn/T/opencode/ref";
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
await page.goto("https://403access.github.io/data-cloud-compare/design-system/os/", { waitUntil: "networkidle" });
await page.waitForTimeout(800);
await page.evaluate(() => {
  document.querySelectorAll(".os-window").forEach((el) => {
    el.classList.remove("is-open", "is-focused");
    el.hidden = true;
  });
  const win = document.querySelector("#win-calendar");
  win.hidden = false;
  win.classList.add("is-open", "is-focused");
});
await page.waitForTimeout(600);
await page.screenshot({ path: `${out}/calendar.png` });
const data = await page.evaluate(() => {
  const pick = (sel) => {
    const el = document.querySelector(sel);
    if (!el) return null;
    const r = el.getBoundingClientRect();
    const s = getComputedStyle(el);
    return { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height), font: s.fontSize, color: s.color, bg: s.backgroundColor, radius: s.borderRadius, padding: s.padding, gap: s.gap };
  };
  return {
    window: pick("#win-calendar"),
    titlebar: pick("#win-calendar .os-titlebar"),
    body: pick("#win-calendar .os-calendar"),
    head: pick("#win-calendar .os-calendar__head"),
    toolbar: pick("#win-calendar .os-calendar__toolbar"),
    month: pick("#win-calendar .os-calendar__month"),
    weekdays: pick("#win-calendar .os-calendar__weekdays"),
    grid: pick("#win-calendar .os-calendar__grid"),
    foot: pick("#win-calendar .os-calendar__foot"),
    html: document.querySelector("#win-calendar").outerHTML.replace(/\n\s*/g, " ").slice(0, 4000),
  };
});
console.log(JSON.stringify(data, null, 2));
await browser.close();
