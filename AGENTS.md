# Agent instructions

Read [README.md](./README.md), [CONTRIBUTING.md](./CONTRIBUTING.md), the consumer guides in
[docs/](./docs/) and the applicable [maintainer notes](./internals/README.md) before changing
behavior. This repository is maintained by agents; useful verification and honest compatibility
claims are essential.

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
- Review how the code is written as well as whether it works: readable, idiomatic Rust, consistent
  structure and naming, appropriate tests, and the conventions of the libraries in use, checked
  against their current documentation. Prefer established practice with a clear benefit over churn.
- A deliberate departure from better practice needs a constraint that rules that practice out, or a
  measured and significant speedup. Explain it in a code comment at the site, with the constraint or
  measured difference and any condition for revisiting it. Treat an unexplained departure as a
  finding.
- When a pull request follows a weak pattern in merged code, examples, templates, or guidance, the
  precedent is the defect. Before settling the original, correct the code and the guidance that
  taught it, as applicable, in separate pull requests. Then refactor the original to the corrected
  pattern or replace it, and link these pull requests to each other.
- When a published security advisory affects a resolved dependency, follow
  [security advisories](./CONTRIBUTING.md#security-advisories) ahead of routine update batching and
  any release-age gate.
- Use regression checks that expose the behavior: complete-predicate fallthrough, capture rollback,
  raw/repeated input, no body reads, committed execution, and service readiness where affected.
- Unexpected failures need triage against the baseline. Do not weaken checks or hide failures to
  obtain green results. Track actionable defects and deferred limits in issues.
- Follow CONTRIBUTING's recurring upstream-test review during early releases. Record relevant
  cases and semantic differences in `internals/upstream-tests.md`; do not mirror entire foreign
  suites.
- Keep human-facing Markdown prose near 100 columns. Preserve unrelated content and formatting.
- Use `.github/labels.json` as the shared label catalog. Every issue and pull request needs one
  `Type:` label and at most one `Status:` label. `Needs:` identifies the person who must act.

## Documentation

- Documentation has three areas: consumer guides in `docs/` (a VitePress site on GitHub Pages),
  the API reference in rustdoc (docs.rs), and maintainer notes in `internals/`. Put each fact
  in the area that owns it and link to it from the others instead of repeating it.
- Update the affected guides, rustdoc and README in the pull request that changes behavior, and
  add an [upgrading](./docs/upgrading.md) section for changes that need consumer action. Verify
  behavior claims against the source and tests.
- Write guides for a newcomer: the concept in plain words, a complete example, then the rules and
  edge cases. Site examples are complete programs with their `use` lines and no rustdoc hidden
  lines, because VitePress shows them as written; `cargo test --package warded-docs` runs them.
  Mark code blocks with one of the languages `docs/pages.rs` allows.
- The site describes the latest release. Keep install pins and version-pinned links at the
  package version; corrections to a released site follow the
  [release procedure](./internals/release.md#correcting-the-released-site).

## Releases

Use SemVer prerelease stages. Alpha is for changes to scope and core behavior. Beta begins when the
agreed feature scope is implemented and the beta gates pass. RC identifies a specific source
candidate intended to become the stable release. See the
[release procedure](./internals/release.md). The owner's hands-on use of `0.1.0-beta.3` is recorded
in [#26](https://github.com/viell-dev/slop-warded-router/issues/26); independent agent usability
reviews cover later candidates and substantial workflow changes.

For the first `0.1.0` release, [issue #8](https://github.com/viell-dev/slop-warded-router/issues/8)
records delegated scope agreement and stage evidence. Beta needs that agreement and fulfilled
criteria. RC and stable each require the owner's explicit approval after review of a concrete
proposal/candidate; review agreement alone does not authorize either transition.

Publish approved releases on crates.io with docs.rs as the versioned API reference. A new channel
requires explicit authorization. Keep unreleased documentation distinct from installable behavior.
Verify exact source, required CI, artifacts, licenses, registry installation and published
documentation before declaring a release complete.
