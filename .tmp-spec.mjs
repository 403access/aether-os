import { chromium } from "@playwright/test";
const b = await chromium.launch();
const p = await b.newPage({ viewport: { width: 1280, height: 800 }, timezoneId: "UTC" });
await p.goto("http://127.0.0.1:8901/os/", { waitUntil: "networkidle" });
await p.waitForTimeout(900);
const data = await p.evaluate(() => {
  const textBox = (el) => {
    const r = document.createRange();
    r.selectNodeContents(el);
    const b = r.getBoundingClientRect();
    return { x: +b.x.toFixed(1), y: +b.y.toFixed(1), w: +b.width.toFixed(1), h: +b.height.toFixed(1) };
  };
  const style = (el) => {
    const s = getComputedStyle(el);
    return { font: s.fontSize, weight: s.fontWeight, line: s.lineHeight, letter: s.letterSpacing, color: s.color, pad: s.padding, h: s.height };
  };
  const box = (el) => { const r = el.getBoundingClientRect(); return { x:+r.x.toFixed(1), y:+r.y.toFixed(1), w:+r.width.toFixed(1), h:+r.height.toFixed(1) }; };
  const menus = [...document.querySelectorAll('.os-menu')];
  const resources = [...document.querySelectorAll('.os-resource')].map(el => ({
    label: el.querySelector('.os-resource__label')?.textContent,
    labelBox: textBox(el.querySelector('.os-resource__label')),
    labelStyle: style(el.querySelector('.os-resource__label')),
    valueBox: textBox(el.querySelector('.os-resource__value')),
    valueStyle: style(el.querySelector('.os-resource__value')),
    meter: box(el.querySelector('.os-resource__meter')),
  }));
  return {
    bar: box(document.querySelector('.os-menubar')),
    barStyle: style(document.querySelector('.os-menubar')),
    app: { box: box(document.querySelector('.os-menubar__app')), text: textBox(document.querySelector('.os-menubar__app')), style: style(document.querySelector('.os-menubar__app')) },
    menus: menus.map(el => ({ label: el.textContent.trim(), box: box(el), text: textBox(el), style: style(el) })),
    sync: { box: box(document.querySelector('.os-status--sync')), text: textBox(document.querySelector('.os-status--sync span:last-child')), style: style(document.querySelector('.os-status--sync')) },
    resources,
    clock: { box: box(document.querySelector('.os-clock')), date: textBox(document.querySelector('[data-clock-date]')), dateStyle: style(document.querySelector('[data-clock-date]')), time: textBox(document.querySelector('[data-clock-time]')), timeStyle: style(document.querySelector('[data-clock-time]')) },
    logo: box(document.querySelector('.os-logo')),
    logoIcon: box(document.querySelector('.os-logo svg')),
    iconButtons: [...document.querySelectorAll('.os-menubar .os-status--icon, .os-menubar .os-menubar__icon')].map(el => ({ label: el.getAttribute('aria-label'), box: box(el), svg: box(el.querySelector('svg')) })),
    windowRim: (() => { const el = document.querySelector('#win-compare'); const s = getComputedStyle(el, '::before'); return { padding: s.padding, bg: s.backgroundImage.slice(0,120), filter: s.filter, opacity: s.opacity, radius: s.borderRadius, animation: s.animation }; })(),
  };
});
console.log(JSON.stringify(data, null, 1));
await b.close();
