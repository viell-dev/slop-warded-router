# Source release procedure

The first-release goal and delegated agreement are recorded in
[issue #8](https://github.com/viell-dev/slop-warded-router/issues/8). The stable 0.1 line is the
router, capture predicates, bounded negotiation, optional Tower/Axum integration and one
representative Leptos application. The server, indexing, typed query/capture storage and universal
adapters stay outside it. Stable 0.1.x preserves the public API, documented routing semantics,
feature names and Rust minimum 1.99.1; intentional breaking changes go to 0.2.0.

## Beta

Beta requires all four agreed gates, with findings reconciled rather than merely counted as votes:

1. Complete declared request-head routing, refinement, negotiation and dispatch behavior; no
   unresolved release-blocking correctness/security defect. Explicitly disposition dependency and
   alpha constraints in [#4](https://github.com/viell-dev/slop-warded-router/issues/4) and
   [#5](https://github.com/viell-dev/slop-warded-router/issues/5).
2. A representative fixture on the candidate: the bounded workflows in its
   [README](../integration/leptos/README.md), complete exact counts, no skips/retries/flakes, a
   deliberately broken predicate, retained logs and success/failure cleanup. Run the seeded input
   corpus in required CI and retain failing inputs. Record affected-change/release trigger policy.
3. A consumer-informed API freeze assessment and one correctness-checked measurement that decides
   whether indexing needs an API change before the freeze. No indexing implementation or arbitrary
   throughput threshold is required. See the [assessment](./beta-assessment.md).
4. Accurate public claims, targeted upstream-test refresh, the Windows/Linux feature/package
   matrix and proportionate independent review of the coherent candidate on its exact revision.

Promote through a PR setting `0.1.0-beta.N` in Cargo/lockfiles, changelog and README, with package
commands deriving the version rather than hardcoding paths. Keep `publish = false`. The review and
required CI must cover the final head. After integration, either rerun candidate evidence on the
`main` commit or verify that its complete tree is identical to the tested/reviewed head. Record
that link; rebase merges can change commit IDs. An optional annotated unprefixed tag may pin the
existing git-source channel. Do not create a crate publication, GitHub Release or new channel.

The owner authorized beta once the agreed gates are met. No additional scope approval is needed
within that goal. Beta is for fixes and polish; a material goal change needs a new scope decision.

## Before RC

RC requires the owner's **explicit approval after review** of a concrete proposal/candidate.
Independent agent agreement alone does not authorize this transition. Before requesting it:

- Finish beta findings and compatibility constraints. Run the [verification procedure](./verification.md)
  on an archive of the exact revision, including packaged examples and dependency policy. Run the
  clean-consumer check against the same pushed commit and record the install, update, removal,
  recovery, features and rustdoc results. Record the commit, tree, toolchain, checks and artifacts
  in the PR. Verify the [GitHub private vulnerability-reporting route](../SECURITY.md). Security
  fixes for the current 0.1.0 prerelease line land on `main`.
- Require exact-candidate success from the hosted [Leptos fixture](../integration/leptos/README.md).
  It runs on pull requests to `main` and pushes to `main` for changes to library source, the root
  `Cargo.toml`, Cargo configuration, the Rust toolchain file, the fixture (except its Markdown) or
  its workflow. The library's `Cargo.lock` is excluded because the fixture has its own lockfile.
  Unprefixed version tags such as `0.1.0-rc.1` and `0.1.0` trigger a run regardless of path. Use
  manual dispatch when candidate evidence is missing and record its tested commit or tree.
- Every pull request reports the required `leptos-fixture` check, alongside the Windows/Linux
  `ci` check. A short job compares the PR merge revision with its base. The check passes only when
  the fixture passed, or when it was skipped because none of those paths changed. Failed, cancelled
  or missing fixture results therefore block merge. Both fixture compilation modes run in the
  hosted job.
- Recommend continuing the personal-use git-source channel for the first RC. Pin a tested commit
  with Cargo's `rev`. An unprefixed annotated tag can name the same commit when authorized.
  The README covers onboarding and links to runnable examples and focused guides. Build rustdoc
  locally from that revision for the corresponding API reference. This meets the source-only goal
  in [#8](https://github.com/viell-dev/slop-warded-router/issues/8#rc-gate-reviewed-concrete-candidate-plus-owner-approval).
  Keep `publish = false`. General distribution needs a separately authorized channel, with a
  documentation site or a justified Rust alternative such as docs.rs. Starting a site or publishing
  to crates.io, docs.rs or GitHub Releases requires explicit owner authorization.
- Prepare the human-use package and assigned open request described below before handing off a
  candidate. The report may arrive during beta or RC, and stable depends on it.
- Prepare and independently review the exact RC proposal, explain evidence limits and remaining
  findings, and link an assigned, self-contained owner approval issue. Do not promote while it waits.

## Git-source handoff

Give consumers the version, full commit SHA and direct source link. Link the changelog, README
and examples at the same commit. Generate rustdoc there with the README command. Verify the
download and its Cargo manifests and lockfiles, retain the archive checksum and check results,
and run the clean-consumer verification with the published `rev`. If an annotated tag is
authorized, confirm it resolves to the verified commit before advertising it.

For a failed candidate or interrupted handoff, keep the previous working `rev` available and
record the failed check. Issue a fresh candidate for a material fix and give consumers its new
SHA and update instructions. Preserve any tag or commit already given to consumers.

## Before stable

Stable separately requires the owner's **explicit approval after review**, plus actual human
installation and use of a representative candidate for the main library tasks. Automated browser
execution, agent reviews and CI cannot satisfy that human gate.

Prepare a small test package with a direct accessible build/source link, installation instructions,
synthetic requests, numbered tasks and expected results. Ask the tester to create/refine a route,
observe fallthrough/captures and use the dispatch boundary, then report the exact build/environment,
what was tried, confusing API/workflow points and untested areas. Do not delegate build engineering
or exhaustive regression work to the person.

Open the human request before handing over the candidate, assign the tester (or owner to arrange
one), use `Needs: Human` and link the stable release it blocks. Keep it open until the actual report
and resulting work are handled. Prepare the separate owner stable-approval request when the
reviewable candidate is complete. RC/stable approval does not silently authorize a new channel.
Routine source work in beta can continue while later-stage requests are pending.
