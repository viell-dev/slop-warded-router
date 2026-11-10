# Agent instructions

Read [README.md](./README.md), [CONTRIBUTING.md](./CONTRIBUTING.md), and any applicable design or
release documentation before changing behavior. This repository is maintained by agents; useful
verification and honest compatibility claims are essential.

## Scope and semantics

- Match the complete declared request-head conditions before selecting one destination. Matchers
  cannot read the request body or trailers. A selected handler's response or failure never retries
  other routes.
- Preserve declaration order across dimensions and overlapping paths. Exhaustion ordinarily
  returns 404; a consumer fallback overrides it. Automatic response policies are deliberate opt-ins.
- Keep pure matching separate from optional Tower dispatch and independent of any server setup.
- Reuse maintained parsing and protocol libraries. Record dependency choices and material semantic
  differences. Copied source needs exact provenance and retained upstream licensing and notices.
- Convenience helpers are ordinary compositions. GET alone does not include HEAD. Secure transport
  is distinct from authentication, and missing security context is distinct from known cleartext.

## Changes and verification

- Use a task branch from `main`. Deliver changes by pull request; keep the default branch protected
  with required CI and no administrator bypass. Prefer rebase integration without rewriting shared
  history.
- Follow Rust ecosystem conventions, the package lint configuration, and `cargo fmt`. Prefer
  narrowly scoped `#[expect(..., reason = "...")]` over blanket lint suppression.
- Run the applicable tests, doctests, feature configurations, Clippy, and documentation build. Review
  the complete diff after checks pass and record commands, environments, limitations, and finding
  dispositions in the pull request.
- Use regression checks that expose the behavior: complete-predicate fallthrough, capture rollback,
  raw/repeated input, no body reads, committed execution, and service readiness where affected.
- Unexpected failures need triage against the baseline. Do not weaken checks or hide failures to
  obtain green results. Track actionable defects and deferred limits in issues.
- Follow CONTRIBUTING's recurring upstream-test review during early releases. Record relevant
  cases and semantic differences in `docs/upstream-tests.md`; do not mirror entire foreign suites.
- Keep human-facing Markdown prose near 100 columns. Preserve unrelated content and formatting.
- Use `.github/labels.json` as the shared label catalog. Every issue and pull request needs one
  `Type:` label and at most one `Status:` label. `Needs:` identifies the person who must act.

## Releases

Until a stable goal is agreed, keep builds in alpha with SemVer prerelease versions. No stable
release may bypass real human use of a representative candidate: prepare install instructions,
synthetic tasks and expected outcomes, assign an open human-testing issue, and record the actual
report and resolution of blockers. Automated checks and agent review do not satisfy that gate.
Repeat focused human testing before stable distribution after substantial workflow changes.

Publishing packages or starting a new distribution channel requires explicit authorization. Keep
unreleased documentation distinct from installable behavior. Verify exact source, required CI,
artifacts, licenses, installation, and published downloads before declaring a release complete.
