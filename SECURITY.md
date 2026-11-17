# Security policy

Warded is available as git source in its `0.1.0` prerelease line. Security fixes are made on `main`;
there are no separately maintained release branches. Pin a tested commit `rev` and update it to
pick up a fix.

## Reporting a vulnerability

Report suspected vulnerabilities privately through GitHub's
[private vulnerability reporting](https://github.com/viell-dev/slop-warded-router/security/advisories/new)
for this repository. Do not open a public issue for an unfixed vulnerability.

Include the commit, Rust version and enabled features, a synthetic request or route definition that
shows the problem, the expected result, the actual result, and the impact you believe it has. Leave
out real credentials, tokens and personal data.

Agents maintain this project; there is no guaranteed response time. A confirmed report is fixed on
`main` with a regression test and disclosed through a GitHub security advisory once a fix is
available. Ordinary defects that are not security problems belong in
[GitHub issues](https://github.com/viell-dev/slop-warded-router/issues).

## Scope

In scope are request-head selection, capture handling, negotiation, and the optional Tower
dispatch boundary: for example a request reaching a route whose declared conditions it does not
meet, a panic or unbounded work caused by request input, or a body read during selection.

Warded implements no HTTP server, TLS, authentication or proxy trust. A `secured()` condition is
only as trustworthy as the `TransportSecurity` value the host application supplies; report faults
in those components to their own projects. The Leptos application under `integration/` is a test
fixture and is not distributed.

The required `ci` check runs `cargo-deny --locked check` against the RustSec advisory database
and enforces the [dependency policy](./deny.toml). Fixes also go through the required
`leptos-fixture` check. See the [release procedure](./docs/release.md#before-rc) for when it
runs the full fixture.
