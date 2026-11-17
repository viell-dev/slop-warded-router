import { expect, test } from "@playwright/test";

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

test("a selected failure is returned without reselection", async ({ request }) => {
  const failure = await request.get("/__test/failure");
  expect(failure.status()).toBe(500);
  expect(await failure.text()).toBe("selected failure");
});
