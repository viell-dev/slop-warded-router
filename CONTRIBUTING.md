# Contributing

Warded is an early Rust library maintained by agents. Small, well-scoped reports, tests, and fixes
are welcome. Establish scope before proposing a new framework, server implementation, or unrelated
feature. Describe behavior with synthetic requests and expected outcomes.

Use a branch and pull request against `main`. Relevant tests and required CI must pass, followed by
a deliberate review of the complete change. Document the Rust version, platforms, commands, and
any checks that remain unavailable. Do not infer compatibility from matching type names or mocks.

Agent-authored issues, pull requests, and reviews must clearly disclose their authorship, the
submitting model and harness, submission on the account owner's behalf, and whether that person
reviewed the material. Authorization and automated or agent verification do not constitute human
review. Use an `Assisted-by: <model> via <harness>` commit trailer for agent assistance; do not add
agent `Co-Authored-By` or `Signed-off-by` trailers.

Keep one `Type:` label on every issue and pull request. An optional `Status:` label expresses the
current disposition; `Needs: Human` or `Needs: Owner` identifies a required response and must be
accompanied by an assignee and self-contained request. See the
[label catalog](https://github.com/viell-dev/slop-warded-router/blob/main/.github/labels.json)
for the catalog.

Contributions to original code are licensed under MIT OR Apache-2.0. Retain applicable notices and
provenance when adapting third-party source; original project licensing does not replace its terms.
