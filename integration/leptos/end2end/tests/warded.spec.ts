import { expect, test } from "@playwright/test";

test.use({ javaScriptEnabled: false });

test("value predicates select ID or username and the handler reads raw captures", async ({ page }) => {
  for (const [value, kind] of [["42", "id"], ["00042", "id"], ["alice", "username"]]) {
    const response = await page.goto(`/users/${value}`);
    if (!response) throw new Error("User page navigation produced no response");
    expect(response.status(), `GET /users/${value}`).toBe(200);
    expect(response.headers()["x-route-kind"]).toBe(kind);
    expect(response.headers()["x-capture"]).toBe(value);
    await expect(page.getByTestId("user-value")).toHaveText(value);
  }
});
