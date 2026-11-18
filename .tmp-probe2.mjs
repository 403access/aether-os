import { chromium } from "@playwright/test";
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
await page.goto("https://403access.github.io/data-cloud-compare/design-system/os/", { waitUntil: "networkidle" });
await page.waitForTimeout(800);
console.log(await page.evaluate(() => {
  const clean = (el) => el.outerHTML.replace(/\n\s*/g, " ").replace(/></g, ">\n<");
  return {
    menubar: clean(document.querySelector(".os-menubar")),
    dock: clean(document.querySelector(".os-dock-wrap")),
    explorerHead: clean(document.querySelector("#win-explorer .os-titlebar")),
  };
}));
await browser.close();
