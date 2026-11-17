import { devices, defineConfig } from "@playwright/test";
import { dirname, join } from "node:path";

const mode = process.env.WARDED_FIXTURE_ROUTER;
if (mode !== "axum" && mode !== "warded") {
  throw new Error("WARDED_FIXTURE_ROUTER must be axum or warded");
}
const report = process.env.WARDED_FIXTURE_REPORT;
const baseURL = process.env.WARDED_FIXTURE_URL;
const revision = process.env.WARDED_FIXTURE_REVISION;
const router = process.env.WARDED_FIXTURE_IDENTITY;
if (!report || !baseURL || !revision || !router) {
  throw new Error("Run check.sh to provide the report path, live server URL and identity, and source revision");
}
const reportDir = dirname(report);
const httpTests = mode === "axum" ? "common.http.spec.ts" : ["common.http.spec.ts", "warded.http.spec.ts"];
const browserTests = mode === "axum" ? "common.spec.ts" : ["common.spec.ts", "warded.spec.ts"];

export default defineConfig({
  testDir: "./tests",
  outputDir: join(reportDir, "test-results"),
  metadata: { revision, router, baseURL },
  timeout: 30000,
  globalTimeout: 240000,
  expect: { timeout: 10000 },
  fullyParallel: false,
  forbidOnly: true,
  retries: 0,
  workers: 1,
  reporter: [
    ["list"],
    ["json", { outputFile: report }],
    ["html", { outputFolder: join(reportDir, "playwright-report"), open: "never" }],
  ],
  use: {
    baseURL,
    actionTimeout: 10000,
    navigationTimeout: 15000,
    trace: "retain-on-failure",
  },
  projects: [
    { name: "http", testMatch: httpTests },
    { name: "chromium", testMatch: browserTests, use: { ...devices["Desktop Chrome"] } },
    { name: "firefox", testMatch: browserTests, use: { ...devices["Desktop Firefox"] } },
    { name: "webkit", testMatch: browserTests, use: { ...devices["Desktop Safari"] } },
  ],
});
