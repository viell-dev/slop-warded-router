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

test("SSR HTML and static assets are served independently of hydration", async ({ request }) => {
  const response = await request.get("/");
  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toContain("text/html");
  expect(await response.text()).toContain("Welcome to Leptos!");
  const css = await request.get("/pkg/warded-leptos-probe.css");
  expect(css.status()).toBe(200);
  expect(css.headers()["content-type"]).toContain("text/css");
  expect(await css.text()).toContain("body");
  expect((await request.get("/favicon.ico")).status()).toBe(200);
});

test("rendered 404, HEAD and explicit method policy retain their HTTP behavior", async ({ request }) => {
  const missing = await request.get("/not-a-route");
  expect(missing.status()).toBe(404);
  expect(await missing.text()).toContain("Page not found.");
  const head = await request.head("/");
  expect(head.status()).toBe(200);
  expect((await head.body()).length).toBe(0);
  const post = await request.post("/");
  expect(post.status()).toBe(405);
  expect(post.headers()["allow"].split(",").map((method) => method.trim()).sort()).toEqual([
    "GET", "HEAD",
  ]);
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

test("a selected failure is returned without reselection", async ({ request }) => {
  const failure = await request.get("/__test/failure");
  expect(failure.status()).toBe(500);
  expect(await failure.text()).toBe("selected failure");
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
