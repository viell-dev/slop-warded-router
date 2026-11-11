import { devices, defineConfig } from "@playwright/test";

const mode = process.env.WARDED_FIXTURE_ROUTER;
if (mode !== "axum" && mode !== "warded") {
  throw new Error("WARDED_FIXTURE_ROUTER must be axum or warded");
}
if (!process.env.WARDED_FIXTURE_REPORT) {
  throw new Error("WARDED_FIXTURE_REPORT must name a fresh JSON report");
}

export default defineConfig({
  testDir: "./tests",
  testMatch: mode === "axum" ? "common.spec.ts" : ["common.spec.ts", "warded.spec.ts"],
  timeout: 30000,
  globalTimeout: 240000,
  expect: { timeout: 10000 },
  fullyParallel: false,
  forbidOnly: true,
  retries: 0,
  workers: 1,
  reporter: [
    ["list"],
    ["json", { outputFile: process.env.WARDED_FIXTURE_REPORT }],
  ],
  use: {
    baseURL: "http://127.0.0.1:3000",
    actionTimeout: 10000,
    navigationTimeout: 15000,
    trace: "retain-on-failure",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "firefox", use: { ...devices["Desktop Firefox"] } },
    { name: "webkit", use: { ...devices["Desktop Safari"] } },
  ],
});
