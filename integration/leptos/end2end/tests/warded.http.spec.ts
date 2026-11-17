import { expect, test } from "@playwright/test";

test("the exact user path rejects invalid, encoded and overflowing values", async ({ request }) => {
  // Send the raw path with the HTTP client. WebKit may decode escaped digits during navigation.
  for (const value of ["42alice", "18446744073709551616", "%34%32", "-1"]) {
    const response = await request.get(`/users/${value}`);
    expect(response.status()).toBe(404);
    expect(response.headers()["x-capture"]).toBeUndefined();
    expect(await response.text()).toBe("No matching user value");
  }
});

test("all header and raw-query conditions must match before an alternative wins", async ({ request }) => {
  const preview = await request.get("/variants?kind=summary&keep=a%2Bb", {
    headers: { "x-mode": "preview" },
  });
  expect(preview.status()).toBe(200);
  expect(await preview.text()).toBe("preview:/variants?kind=summary&keep=a%2Bb");
  expect(preview.headers()["vary"].split(",").map((token) => token.trim())).toContain("x-mode");
  for (const [url, headers] of [
    ["/variants?kind=summary", {}],
    ["/variants?kind=detail", { "x-mode": "preview" }],
    ["/variants?kind=%73ummary", { "x-mode": "preview" }],
  ] as const) {
    const response = await request.get(url, { headers });
    expect(response.status()).toBe(200);
    expect(await response.text()).toBe("default");
  }
});
