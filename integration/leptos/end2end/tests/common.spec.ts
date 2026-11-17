import { expect, test } from "@playwright/test";

test("SSR hydrates into an interactive application", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  expect((await page.goto("/"))?.status()).toBe(200);
  await expect(page).toHaveTitle("Welcome to Leptos");
  await expect(page.locator("h1")).toHaveText("Welcome to Leptos!");
  await expect(page.locator("main")).toHaveAttribute("data-hydrated", "true");
  await page.getByRole("button", { name: "Click Me: 0", exact: true }).click();
  await expect(page.getByRole("button", { name: "Click Me: 1", exact: true })).toBeVisible();
  expect(errors).toEqual([]);
});

test("full dynamic requests and hydrated client navigation have distinct boundaries", async ({ page }) => {
  let documents = 0;
  page.on("request", (request) => {
    if (request.isNavigationRequest() && request.frame() === page.mainFrame()) documents += 1;
  });
  expect((await page.goto("/users/42"))?.status()).toBe(200);
  await expect(page.getByTestId("user-value")).toHaveText("42");
  await expect(page.locator("main")).toHaveAttribute("data-hydrated", "true");
  expect(documents).toBe(1);
  await page.getByRole("link", { name: "Named user", exact: true }).click();
  await expect(page).toHaveURL(/\/users\/alice$/);
  await expect(page.getByTestId("user-value")).toHaveText("alice");
  expect(documents).toBe(1);
  expect((await page.reload())?.status()).toBe(200);
  await expect(page.getByTestId("user-value")).toHaveText("alice");
  expect(documents).toBe(2);
});

test.describe("server-rendered fallback", () => {
  test.use({ javaScriptEnabled: false });

  test("a trailing-slash user path renders through the fallback with 404", async ({ page }) => {
    const response = await page.goto("/users/-1/");
    expect(response?.status()).toBe(404);
    expect(response?.headers()["x-capture"]).toBeUndefined();
    await expect(page.getByTestId("user-value")).toHaveText("-1");
  });
});

test("a real form preserves its body and returns server-function errors", async ({ page }) => {
  await page.goto("/form");
  await expect(page.locator("main")).toHaveAttribute("data-hydrated", "true");
  const message = "A + B & Café = /";
  await page.getByLabel("Message", { exact: true }).fill(message);
  const success = page.waitForResponse((response) => response.url().endsWith("/api/echo-message")
    && response.request().method() === "POST");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  expect((await success).status()).toBe(200);
  await expect(page.getByTestId("form-result")).toHaveText(`Echo: ${message}`);
  await page.getByLabel("Message", { exact: true }).fill("reject");
  const failure = page.waitForResponse((response) => response.url().endsWith("/api/echo-message")
    && response.request().method() === "POST");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  expect((await failure).status()).toBe(500);
  await expect(page.getByTestId("form-result")).toContainText("synthetic form failure");
  await expect(page.getByTestId("form-result")).not.toContainText("incorrect");
});

test("SSR initial content is readable before late content is released", async ({ page, request }) => {
  expect((await page.goto("/stream", { waitUntil: "commit" }))?.status()).toBe(200);
  // No wall-clock threshold: the server cannot complete the late resource until
  // this test releases it, after observing the first rendered bytes.
  await expect(page.getByTestId("stream-start")).toHaveText("Stream started");
  await expect(page.getByTestId("stream-late")).toHaveCount(0);
  expect((await request.post("/__test/release-stream")).status()).toBe(204);
  await expect(page.getByTestId("stream-late")).toHaveText("Late content arrived");
  await expect(page.locator("main")).toHaveAttribute("data-hydrated", "true");
});
