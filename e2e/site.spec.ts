import { expect, test, type ConsoleMessage } from "@playwright/test";

function trackConsoleErrors(page: import("@playwright/test").Page): string[] {
  const errors: string[] = [];
  page.on("console", (message: ConsoleMessage) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(error.message));
  return errors;
}

test.describe("marketing site", () => {
  test("renders the hero and primary navigation", async ({ page }) => {
    const errors = trackConsoleErrors(page);
    await page.goto("/");

    await expect(page.getByRole("heading", { level: 1 })).toContainText("one user");
    await expect(page.getByRole("link", { name: "Read the architecture" })).toBeVisible();
    await expect(page.getByRole("navigation", { name: "Primary" })).toBeVisible();
    await expect(page.locator(".hero__stats")).toContainText("≤ 200 MB");
    expect(errors).toEqual([]);
  });

  test("filters the live palette and executes a result", async ({ page }) => {
    await page.goto("/#palette");

    const input = page.getByLabel("Search", { exact: true });
    await input.click();
    await input.fill("settings");

    const options = page.getByRole("option");
    await expect(options.first()).toBeVisible();
    await expect(page.locator(".aether-palette")).toContainText("Settings");

    await options.first().click();
    await expect(page.locator(".palette-card__result")).toContainText("aetherd");
  });

  test("mode prefixes and fuzzy typos find the right documents", async ({ page }) => {
    await page.goto("/#palette");
    const input = page.getByLabel("Search", { exact: true });

    await page.getByRole("button", { name: "files" }).click();
    await expect(input).toHaveValue("/index");
    await expect(page.locator(".aether-palette")).toContainText("indexer.rs");

    await page.getByRole("button", { name: "typo" }).click();
    await expect(input).toHaveValue("compositr");
    await expect(page.locator(".aether-palette")).toContainText("compositor.rs");
  });

  test("supports keyboard navigation in the palette", async ({ page }) => {
    await page.goto("/#palette");
    const input = page.getByLabel("Search", { exact: true });
    await input.click();
    await input.fill("aether");

    const first = page.getByRole("option").first();
    await expect(first).toHaveAttribute("aria-selected", "true");
    await input.press("ArrowDown");

    const selected = page.locator('[role="option"][aria-selected="true"]');
    await expect(selected).toHaveCount(1);
    await input.press("Enter");
    await expect(page.locator(".palette-card__result")).toContainText("aetherd");
  });

  test("navigates the two-pane explorer", async ({ page }) => {
    await page.goto("/#explorer");

    const home = page.getByRole("listbox", { name: "Home entries" });
    await expect(home.getByRole("option")).toHaveCount(7);

    await home.getByRole("option", { name: /projects/ }).dblclick();
    await expect(page.locator(".aether-pane").first()).toContainText("/home/oliver/projects");
    await expect(page.getByRole("listbox", { name: "Home entries" })).toContainText("aether-os");

    await page.getByRole("button", { name: "Back" }).first().click();
    await expect(page.locator(".aether-pane").first()).toContainText("/home/oliver");
  });

  test("explorer search filters rows", async ({ page }) => {
    await page.goto("/#explorer");
    const search = page.getByLabel("Search Home");
    await search.fill("notes");
    const home = page.getByRole("listbox", { name: "Home entries" });
    await expect(home.getByRole("option")).toHaveCount(1);
    await expect(home).toContainText("notes");
  });

  test("links to the docs", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("link", { name: "Docs", exact: true }).click();
    await expect(page).toHaveURL(/\/docs\/$/);
    await expect(page.getByRole("heading", { level: 1 })).toContainText("Aether OS");
  });
});

test.describe("documentation", () => {
  test("loads the docs home and sidebar", async ({ page }) => {
    const errors = trackConsoleErrors(page);
    await page.goto("/docs/");

    await expect(page.getByRole("heading", { level: 1 })).toContainText("Aether OS");
    await expect(page.getByRole("heading", { name: "What exists today" })).toBeVisible();
    await expect(
      page.getByRole("link", { name: "Product requirements", exact: true }),
    ).toBeVisible();
    expect(errors).toEqual([]);
  });

  test("navigates to a subsystem page", async ({ page }) => {
    await page.goto("/docs/");
    await page.getByRole("link", { name: "Product requirements", exact: true }).click();
    await expect(page).toHaveURL(/\/docs\/product\/prd\/?$/);
    await expect(page.getByRole("heading", { level: 1 })).toContainText("Product requirements");
    await expect(page.getByRole("heading", { name: "Release criteria for v1.0" })).toBeVisible();
  });

  test("serves the claims and evidence page", async ({ page }) => {
    await page.goto("/docs/reference/claims-and-evidence/");
    await expect(page.getByRole("heading", { level: 1 })).toContainText("Claims and evidence");
    await expect(page.getByText("Refused").first()).toBeVisible();
  });

  test("serves the design system gallery", async ({ page }) => {
    await page.goto("/design-system/");
    await expect(page.getByRole("heading", { level: 1 })).toContainText("Water");
    await expect(page.locator(".ds-btn").first()).toBeVisible();
  });
});
