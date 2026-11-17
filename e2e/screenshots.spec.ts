import { mkdir } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test, type Page } from "@playwright/test";
import {
  installPeerConnectionDouble,
  stubHealth,
  stubSignaling,
} from "./viewer-stubs";

/**
 * Regenerate the browser-facing screenshots.
 *
 *   UPDATE_SCREENSHOTS=1 bunx playwright test e2e/screenshots.spec.ts
 *
 * The normal e2e run skips this spec. `scripts/capture-web-screenshots.sh`
 * builds the assembled site first so the captures never describe stale dist
 * output.
 */
const UPDATE = process.env.UPDATE_SCREENSHOTS === "1";
const OUT = resolve(process.env.SCREENSHOT_OUT ?? "artifacts/screenshots/web");

async function shot(page: Page, name: string): Promise<void> {
  await page.screenshot({ path: join(OUT, `${name}.png`), animations: "disabled" });
}

async function element(page: Page, selector: string, name: string): Promise<void> {
  const target = page.locator(selector).first();
  await target.scrollIntoViewIfNeeded();
  // Sticky navigation would otherwise sit in the middle of a tall section
  // capture at whatever scroll offset Playwright chose.
  await page.addStyleTag({
    content: ".site-nav,.show-top,.show-nav{visibility:hidden!important}",
  });
  await target.screenshot({
    path: join(OUT, `${name}.png`),
    animations: "disabled",
  });
}

async function safeFullPage(page: Page, name: string): Promise<void> {
  try {
    await page.screenshot({
      path: join(OUT, `${name}.png`),
      fullPage: true,
      animations: "disabled",
    });
  } catch {
    // Very tall docs pages can exceed Chromium's texture limit. Keep the
    // viewport capture so the run still produces a useful image.
    await shot(page, name);
  }
}

async function settle(page: Page): Promise<void> {
  await page.addStyleTag({
    content:
      "*,*::before,*::after{animation:none!important;transition:none!important;caret-color:transparent!important}",
  });
  await page.evaluate(async () => {
    const step = Math.max(320, Math.round(window.innerHeight * 0.75));
    const end = document.documentElement.scrollHeight;
    for (let y = 0; y < end; y += step) {
      window.scrollTo(0, y);
      await new Promise((done) => window.setTimeout(done, 40));
    }
    window.scrollTo(0, 0);
    await document.fonts.ready;
  });
  await page.waitForTimeout(250);
}

test.skip(!UPDATE, "set UPDATE_SCREENSHOTS=1 to regenerate the screenshot gallery");

test.use({
  viewport: { width: 1440, height: 1000 },
  deviceScaleFactor: 1,
  colorScheme: "dark",
  reducedMotion: "reduce",
});

test.beforeAll(async () => {
  await mkdir(OUT, { recursive: true });
});

test("marketing site", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("one user");
  await settle(page);

  await safeFullPage(page, "marketing-home");
  await element(page, "header.hero", "marketing-hero");
  await element(page, "#palette", "marketing-palette");
  await element(page, "#explorer", "marketing-explorer");
  await element(page, "#screenshots", "marketing-screenshots");
  await element(page, "#architecture", "marketing-architecture");
  await element(page, "#isolation", "marketing-isolation");
  await element(page, "#remote", "marketing-remote");
  await element(page, "#design", "marketing-design");
  await element(page, "#roadmap", "marketing-roadmap");
  await element(page, "#performance", "marketing-performance");

  // The gallery lightbox is a UI surface of its own.
  await page.goto("/#screenshots");
  await settle(page);
  await page.getByRole("button", { name: "Open screenshot: Command palette" }).click();
  await expect(page.locator(".lightbox")).toBeVisible();
  await page.waitForTimeout(150);
  await shot(page, "marketing-lightbox");
});

test("remote viewer and live simulation", async ({ page }) => {
  await page.goto("/#remote");
  await expect(page.getByRole("heading", { name: "The desktop, in a tab." })).toBeVisible();
  await settle(page);

  const session = page.locator(".remote-pair__state");
  await expect(session).toContainText("Waiting for pairing");
  await page.getByRole("button", { name: "Start session" }).click();
  await expect(session).toContainText("Streaming · session rs-demo-kestrel", {
    timeout: 10_000,
  });
  await page.waitForTimeout(700);
  await element(page, "#remote", "marketing-remote-live");
  await element(page, ".remote-window", "remote-window");
  await shot(page, "marketing-remote-panel");
});

test("remote viewer page states", async ({ page }) => {
  await installPeerConnectionDouble(page);
  await stubSignaling(page, "K7QM2H4P");
  await stubHealth(page, "synthetic");

  await page.goto("/viewer.html");
  await expect(page.getByRole("heading", { name: "The desktop, in a tab." })).toBeVisible();
  await settle(page);
  await shot(page, "viewer-idle");
  await element(page, ".viewer__stage", "viewer-stage-idle");

  await page.goto("/viewer.html?code=K7QM2H4P");
  await page.getByRole("button", { name: "Connect" }).click();
  await expect(page.getByRole("status")).toContainText("Streaming · session rs-viewer-e2e", {
    timeout: 10_000,
  });
  await expect(page.getByTestId("viewer-hud")).toContainText("640×360", { timeout: 5_000 });
  await page.waitForTimeout(400);
  await shot(page, "viewer-live");
  await element(page, ".viewer__stage", "viewer-stage-live");

  await page.goto("/viewer.html?code=ZZZZZZZZ");
  await page.getByRole("button", { name: "Connect" }).click();
  await expect(page.getByRole("alert")).toContainText("invalid pairing code");
  await page.waitForTimeout(200);
  await shot(page, "viewer-error");
});

test("documentation pages", async ({ page }) => {
  const pages: Array<{ name: string; path: string; full?: boolean }> = [
    { name: "docs-home", path: "/docs/", full: true },
    { name: "docs-architecture", path: "/docs/architecture/smithay-backend/" },
    { name: "docs-design-system", path: "/docs/design/design-system/" },
    { name: "docs-using-aether", path: "/docs/getting-started/using-aether/" },
    { name: "docs-virtual-machine", path: "/docs/getting-started/virtual-machine/" },
    { name: "docs-roadmap", path: "/docs/roadmap/" },
    { name: "docs-evidence", path: "/docs/reference/claims-and-evidence/" },
  ];

  for (const entry of pages) {
    await page.goto(entry.path);
    await settle(page);
    if (entry.full === true) {
      await safeFullPage(page, entry.name);
    } else {
      await shot(page, entry.name);
    }
  }
});

test("design system gallery", async ({ page }) => {
  await page.goto("/design-system/");
  await expect(page.locator(".show-hero")).toBeVisible();
  await settle(page);
  await safeFullPage(page, "design-system-home");
  await element(page, ".show-hero", "design-system-hero");
  await element(page, "#foundations", "design-system-foundations");
  await element(page, "#components", "design-system-components");
  await element(page, "#water", "design-system-water");
});

test.describe("mobile", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test("marketing and viewer", async ({ page }) => {
    await page.goto("/");
    await settle(page);
    await shot(page, "mobile-home");
    await element(page, "#screenshots", "mobile-screenshots");

    await page.goto("/viewer.html");
    await expect(page.getByRole("heading", { name: "The desktop, in a tab." })).toBeVisible();
    await settle(page);
    await shot(page, "mobile-viewer");
  });
});