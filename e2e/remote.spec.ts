import { expect, test } from "@playwright/test";

test.describe("remote display viewer", () => {
  test("pairs, streams, shows the HUD and stops cleanly", async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => {
      if (message.type() === "error") errors.push(message.text());
    });

    await page.goto("/#remote");
    await expect(page.getByRole("heading", { name: "The desktop, in a tab." })).toBeVisible();
    await expect(page.getByLabel("Pairing code")).toHaveText("K7QM-2H4P");
    const sessionState = page.locator(".remote-pair__state");
    await expect(sessionState).toContainText("Waiting for pairing");

    const canvas = page.getByLabel("Simulated Aether desktop streamed into the browser client");
    await expect(canvas).toBeVisible();
    const initialWidth = await canvas.evaluate((element) => (element as HTMLCanvasElement).width);
    expect(initialWidth).toBeGreaterThan(0);

    await page.getByRole("button", { name: "Start session" }).click();
    await expect(sessionState).toContainText("Streaming · session rs-demo-kestrel", {
      timeout: 10_000,
    });
    const hud = page.getByTestId("remote-hud");
    await expect(hud).toContainText("WebRTC");
    await expect(hud).toContainText("H264");
    await expect(hud).toContainText("frames");
    await expect(page.locator(".remote-rec")).toContainText("recording");

    await page.waitForTimeout(600);
    const first = await hud.textContent();
    await page.waitForTimeout(300);
    const second = await hud.textContent();
    expect(second).not.toBe(first);

    if (process.env.UPDATE_SCREENSHOTS === "1") {
      await page.locator("#remote").screenshot({
        path: "apps/docs/static/img/remote-viewer.png",
      });
      await page.locator(".remote-window").screenshot({
        path: "apps/web/public/screenshots/remote.png",
      });
    }

    await page.getByRole("button", { name: "Stop" }).click();
    await expect(sessionState).toContainText("Session closed");
    await expect(page.locator(".remote-rec")).toContainText("ready");
    expect(errors).toEqual([]);
  });

  test("explains the session policy and links to the evidence", async ({ page }) => {
    await page.goto("/#remote");
    const facts = page.locator(".remote-pair__facts");
    await expect(facts).toContainText("scancodes only");
    await expect(facts).toContainText("off by default");
    await expect(
      page.getByRole("link", { name: "claims and evidence" }),
    ).toHaveAttribute("href", "/docs/reference/claims-and-evidence/");
    await expect(page.getByRole("link", { name: "live viewer" })).toHaveAttribute(
      "href",
      "/viewer.html",
    );
  });
});
