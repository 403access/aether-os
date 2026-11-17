import { expect, test } from "@playwright/test";
import {
  installPeerConnectionDouble,
  stubHealth,
  stubSignaling,
} from "./viewer-stubs";

test.describe("live viewer", () => {
  test("pairs, streams, updates the HUD, sends input and stops", async ({ page }) => {
    // The fake transport is fast, but a loaded CI runner can still need more
    // than the suite default while it negotiates and samples the HUD.
    test.setTimeout(45_000);
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => {
      if (message.type() === "error") errors.push(message.text());
    });
    await installPeerConnectionDouble(page);
    await stubSignaling(page, "K7QM2H4P");
    await stubHealth(page, "synthetic");

    await page.goto("/viewer.html?code=K7QM2H4P");
    await expect(page.getByRole("heading", { name: "The desktop, in a tab." })).toBeVisible();
    await expect(page.getByLabel("Pairing code")).toHaveValue("K7QM2H4P");
    await expect(page.getByLabel("Signaling URL")).toHaveValue(
      "http://127.0.0.1:7443/signal",
    );
    await expect(page.getByRole("status")).toContainText("Waiting for pairing");

    await page.getByRole("button", { name: "Connect" }).click();
    await expect(page.getByRole("status")).toContainText(
      "Streaming · session rs-viewer-e2e",
      { timeout: 10_000 },
    );

    const hud = page.getByTestId("viewer-hud");
    await expect(hud).toContainText("webrtc");
    await expect(hud).toContainText("h264");
    await expect(hud).toContainText("open");
    await expect(hud).toContainText("640×360", { timeout: 5_000 });
    await expect(hud).toContainText("240");
    await expect(page.getByTestId("viewer-source")).toContainText(
      "synthetic test source",
    );
    await expect(page.getByTestId("viewer-hud-source")).toContainText(
      "synthetic test source",
    );
    await expect(page.locator(".viewer__rec")).toContainText("recording");

    await page.getByLabel("Send input").check();
    // The viewer ignores keys typed into form controls. Move focus off the
    // checkbox, then use the real keyboard path so the test proves the page,
    // the batcher and the data channel together.
    await page.evaluate(() => {
      (document.activeElement as HTMLElement | null)?.blur();
    });
    await page.keyboard.press("a");
    await page.waitForFunction(
      () =>
        (window as unknown as { __viewerInput: string[] }).__viewerInput.some(
          (entry) => entry.includes('"scancode":30'),
        ),
      undefined,
      { timeout: 10_000 },
    );

    // The video surface owns pointer input in a live session. Activate the
    // button directly so the lifecycle check cannot be intercepted by it.
    await page.getByRole("button", { name: "Stop" }).evaluate((button) => {
      (button as HTMLButtonElement).click();
    });
    await expect(page.getByRole("status")).toContainText("Session closed");
    await expect(page.locator(".viewer__rec")).toContainText("ready");
    expect(
      await page.evaluate(
        () => (window as unknown as { __viewerClosed: boolean }).__viewerClosed,
      ),
    ).toBe(true);
    expect(errors).toEqual([]);
  });

  test("shows the host's pairing error", async ({ page }) => {
    await installPeerConnectionDouble(page);
    await stubSignaling(page, "K7QM2H4P");
    await stubHealth(page, "synthetic");

    await page.goto("/viewer.html?code=ZZZZZZZZ");
    await page.getByRole("button", { name: "Connect" }).click();
    await expect(page.getByRole("alert")).toContainText("invalid pairing code");
    await expect(page.getByRole("status")).toContainText("Session error");
  });
});
