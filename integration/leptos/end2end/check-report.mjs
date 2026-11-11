import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const [file, mode] = process.argv.slice(2);
assert.ok(["axum", "warded", "negative"].includes(mode), "unknown report mode");
const report = JSON.parse(readFileSync(file, "utf8"));
const cases = [];
function visit(suite) {
  for (const spec of suite.specs ?? []) {
    for (const test of spec.tests) cases.push({ title: spec.title, ...test });
  }
  for (const child of suite.suites ?? []) visit(child);
}
for (const suite of report.suites) visit(suite);
assert.deepEqual(report.errors, [], "runner errors are failures");
const count = mode === "axum" ? 7 : mode === "warded" ? 9 : 1;
assert.equal(cases.length, count * 3, "missing, extra or crashed tests");
assert.equal(report.stats.skipped, 0);
assert.equal(report.stats.flaky, 0);
assert.equal(report.stats.expected, mode === "negative" ? 0 : count * 3);
assert.equal(report.stats.unexpected, mode === "negative" ? 3 : 0);
for (const browser of ["chromium", "firefox", "webkit"]) {
  const tests = cases.filter((test) => test.projectName === browser);
  assert.equal(tests.length, count, `incomplete ${browser} run`);
  assert.equal(new Set(tests.map((test) => test.title)).size, count, "duplicate cases");
}
for (const test of cases) {
  assert.equal(test.expectedStatus, "passed", "expected failures cannot hide a defect");
  assert.equal(test.results.length, 1, "retries are failures");
  assert.equal(test.results[0].retry, 0);
  assert.equal(test.results[0].status, mode === "negative" ? "failed" : "passed");
  if (mode === "negative") {
    assert.equal(test.title, "value predicates select ID or username and the handler reads raw captures");
    const message = test.results[0].error?.message?.replace(/\u001b\[[0-9;]*m/g, "") ?? "";
    assert.match(message, /Expected: 200/);
    assert.match(message, /Received: 404/);
  }
}
console.log(`Verified ${mode}: ${cases.length} complete results, no retries/skips/flakes`);
