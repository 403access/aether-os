import { chromium } from "@playwright/test";
const out = "/private/var/folders/q1/drdd6kwj19z0l36drkth_y940000gn/T/opencode/ref";
const browser = await chromium.launch();
for (const [name, w, h] of [["os-1280", 1280, 800], ["os-1440", 1440, 900]]) {
  const page = await browser.newPage({ viewport: { width: w, height: h }, deviceScaleFactor: 1 });
  await page.goto("https://403access.github.io/data-cloud-compare/design-system/os/", { waitUntil: "networkidle" });
  await page.waitForTimeout(2500);
  await page.screenshot({ path: `${out}/${name}.png` });
  await page.close();
}
await browser.close();
