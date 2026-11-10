#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export CI=1 NO_COLOR=1 PLAYWRIGHT_JSON_OUTPUT_NAME=results.json
rustc --version
node --version
cargo leptos --version
node -e 'require("node:fs").rmSync("end2end/results.json", { force: true })'
npm --prefix end2end ci --ignore-scripts --no-audit --no-fund
cargo leptos end-to-end --cargo-locked
node <<'JS'
const assert = require("node:assert/strict");
const report = require("./end2end/results.json");
assert.equal(report.stats.expected, 9, "all nine browser checks must run and pass");
for (const name of ["unexpected", "flaky", "skipped"]) {
  assert.equal(report.stats[name], 0, name);
}
console.log("Verified complete Playwright result: 9 passed, no retries or skips.");
JS
