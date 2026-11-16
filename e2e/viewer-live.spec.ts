import { expect, test } from "@playwright/test";

/**
 * Opt-in live check against a real `aether-remote serve`.
 *
 *   aether-remote serve --http 127.0.0.1:7443 --code K7QM2H4P --seconds 120
 *   AETHER_REMOTE_LIVE=1 bunx playwright test e2e/viewer-live.spec.ts
 *
 * System Chrome is required: Playwright's bundled Chromium often omits the
 * proprietary H.264 decoder, which is the only codec the host ships today.
 */
const live = process.env.AETHER_REMOTE_LIVE === "1";

test.use({ channel: "chrome" });

test.describe("live viewer against aether-remote", () => {
  test.skip(!live, "set AETHER_REMOTE_LIVE=1 with a host already running");

  test("pairs, negotiates and decodes real H.264 frames", async ({ page }) => {
    test.setTimeout(45_000);
    const code = process.env.AETHER_REMOTE_CODE ?? "K7QM2H4P";
    const signal = process.env.AETHER_REMOTE_SIGNAL ?? "http://127.0.0.1:7443/signal";

    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));

    await page.goto(
      `/viewer.html?code=${encodeURIComponent(code)}&signal=${encodeURIComponent(signal)}`,
    );
    await page.getByRole("button", { name: "Connect" }).click();

    await expect(page.getByRole("status")).toContainText("Streaming · session rs-", {
      timeout: 15_000,
    });
    await expect(page.getByTestId("viewer-hud")).toContainText("640×360", {
      timeout: 15_000,
    });
    await expect(page.getByTestId("viewer-source")).toHaveText(
      /synthetic test source|live desktop/,
    );
    await expect(page.locator(".viewer__rec")).toContainText("recording");

    const video = await page
      .getByLabel("Live Aether desktop streamed into the browser")
      .evaluate((element) => {
        const media = element as HTMLVideoElement;
        return {
          width: media.videoWidth,
          height: media.videoHeight,
          currentTime: media.currentTime,
        };
      });
    expect(video.width).toBe(640);
    expect(video.height).toBe(360);
    expect(video.currentTime).toBeGreaterThan(0);

    await page.getByRole("button", { name: "Stop" }).click();
    await expect(page.getByRole("status")).toContainText("Session closed");
    expect(errors).toEqual([]);
  });
});
