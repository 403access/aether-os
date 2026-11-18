import { chromium } from "@playwright/test";
const b = await chromium.launch();
const p = await b.newPage({ viewport: { width: 1280, height: 800 }, timezoneId: "UTC" });
await p.goto("http://127.0.0.1:8901/os/", { waitUntil: "networkidle" });
await p.waitForTimeout(800);
const apps = ["compare","explorer","settings","network","battery","devices","calendar","taskmanager","terminal","music","shortcuts","about","builder"];
const out = {};
for (const id of apps) {
  await p.evaluate(() => document.querySelectorAll('.os-window.is-open [data-window-action="close"]').forEach(b=>b.click()));
  const cmd = id === "taskmanager" ? "open-taskmanager" : id === "builder" ? "open-builder" : "open-" + id;
  await p.evaluate((c) => {
    const els=[...document.querySelectorAll(`[data-cmd="${c}"]`)];
    const vis=els.find(e=>e.getClientRects().length>0 && !e.closest('[hidden]')) || els[0];
    vis?.click();
  }, cmd);
  await p.waitForTimeout(500);
  out[id] = await p.evaluate((wid) => {
    const el=document.querySelector(`#win-${wid}`);
    if(!el) return null;
    const r=el.getBoundingClientRect();
    const s=getComputedStyle(el);
    const title=el.querySelector('.os-titlebar__title');
    return {
      rect:[Math.round(r.x),Math.round(r.y),Math.round(r.width),Math.round(r.height)],
      title:title?.querySelector('b')?.textContent?.trim(),
      sub:title?.querySelector('.os-titlebar__sub')?.textContent?.trim(),
      bg:s.backgroundColor,
      bodyClass: el.firstElementChild?.nextElementSibling?.className,
      headings:[...el.querySelectorAll('h2,h3,.os-app-section__title,dt,legend')].slice(0,14).map(x=>x.textContent.trim().replace(/\s+/g,' ')).filter(Boolean),
      rows:[...el.querySelectorAll('.os-row, .os-setting-row, .os-device-row, .os-network-row')].slice(0,6).map(x=>x.textContent.trim().replace(/\s+/g,' ').slice(0,80)),
    };
  }, id);
}
console.log(JSON.stringify(out,null,1));
await b.close();
