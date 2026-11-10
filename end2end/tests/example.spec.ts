import { test, expect } from "@playwright/test";

test("homepage has title and heading text", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const response = await page.goto("http://localhost:3000/");

  expect(response?.status()).toBe(200);
  await expect(page).toHaveTitle("Welcome to Leptos");
  await expect(page.locator("h1")).toHaveText("Welcome to Leptos!");
  // This static fixture loads its hydration code through a dynamic JS/WASM import.
  await page.waitForLoadState("networkidle");
  const counter = page.getByRole("button", { name: "Click Me: 0" });
  await counter.click();
  await expect(page.getByRole("button", { name: "Click Me: 1" })).toBeVisible();
  expect(errors).toEqual([]);
});

test("server renders without JavaScript and serves the stylesheet", async ({ request }) => {
  const response = await request.get("http://localhost:3000/");
  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toContain("text/html");
  const html = await response.text();
  expect(html).toContain("Welcome to Leptos!");
  expect(html).toContain("Click Me:");
  const css = await request.get("http://localhost:3000/pkg/warded-leptos-probe.css");
  expect(css.status()).toBe(200);
  expect(css.headers()["content-type"]).toContain("text/css");
  expect(await css.text()).toContain("body");
});

test("fallbacks, HEAD and method refusal retain their HTTP behavior", async ({ request }) => {
  const missing = await request.get("http://localhost:3000/not-a-route");
  expect(missing.status()).toBe(404);
  expect(await missing.text()).toContain("Page not found.");
  const favicon = await request.get("http://localhost:3000/favicon.ico");
  expect(favicon.status()).toBe(200);
  const head = await request.head("http://localhost:3000/");
  expect(head.status()).toBe(200);
  expect((await head.body()).length).toBe(0);
  const post = await request.post("http://localhost:3000/");
  expect(post.status()).toBe(405);
  expect(post.headers()["allow"].split(",").map((method) => method.trim()).sort()).toEqual([
    "GET",
    "HEAD",
  ]);
});
