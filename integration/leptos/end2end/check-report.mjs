import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const [file, variant] = process.argv.slice(2);
assert.ok(["axum", "warded", "negative"].includes(variant), "unknown report variant");
const negative = variant === "negative";
const httpCount = { axum: 3, warded: 5, negative: 0 }[variant];
const browserCount = { axum: 5, warded: 6, negative: 1 }[variant];
const expectedTotal = httpCount + 3 * browserCount;
const report = JSON.parse(readFileSync(file, "utf8"));
const { revision, router, baseURL } = report.config.metadata;
assert.equal(revision, process.env.WARDED_FIXTURE_REVISION, "report source revision");
assert.equal(router, process.env.WARDED_FIXTURE_IDENTITY, "report live router identity");
assert.equal(baseURL, process.env.WARDED_FIXTURE_URL, "report live server URL");
const cases = [];
function visit(suite) {
  for (const spec of suite.specs ?? []) {
    for (const test of spec.tests) cases.push({ title: spec.title, ...test });
  }
  for (const child of suite.suites ?? []) visit(child);
}
for (const suite of report.suites) visit(suite);
assert.deepEqual(report.errors, [], "runner errors are failures");
assert.equal(cases.length, expectedTotal, "missing, extra or crashed tests");
assert.equal(report.stats.skipped, 0);
assert.equal(report.stats.flaky, 0);
assert.equal(report.stats.expected, negative ? 0 : expectedTotal);
assert.equal(report.stats.unexpected, negative ? expectedTotal : 0);
for (const [project, count] of Object.entries({
  http: httpCount,
  chromium: browserCount,
  firefox: browserCount,
  webkit: browserCount,
})) {
  const tests = cases.filter((test) => test.projectName === project);
  assert.equal(tests.length, count, `incomplete ${project} run`);
  assert.equal(new Set(tests.map((test) => test.title)).size, count, "duplicate cases");
}
for (const test of cases) {
  assert.equal(test.expectedStatus, "passed", "expected failures cannot hide a defect");
  assert.equal(test.results.length, 1, "retries are failures");
  assert.equal(test.results[0].retry, 0);
  assert.equal(test.results[0].status, negative ? "failed" : "passed");
  if (negative) {
    assert.equal(test.title, "value predicates select ID or username and the handler reads raw captures");
    const message = test.results[0].error?.message?.replace(/\u001b\[[0-9;]*m/g, "") ?? "";
    assert.match(message, /GET \/users\/42/);
    assert.match(message, /Expected: 200/);
    assert.match(message, /Received: 404/);
  }
}
const outcome = negative ? "expected failures" : "passes";
console.log(`Verified ${variant}: ${httpCount} HTTP-client and ${browserCount} per engine (${outcome}), no retries/skips/flakes`);
