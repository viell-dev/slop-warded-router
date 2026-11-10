# Verification

Use Rust 1.99.1 from `rust-toolchain.toml`. CI requires Windows and Linux checks and a successful
aggregate `ci` result before integration.

```sh
cargo fmt --all -- --check
cargo clippy --all-features --all-targets --locked -- -D warnings
cargo test --all-features --locked
cargo test --no-default-features --locked
cargo test --no-default-features --features tower --locked
cargo test --no-default-features --features negotiation --locked
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --locked
cargo run --example basic --locked
cargo run --example server --locked -- --check
cargo package --locked
cargo run --manifest-path target/package/warded-0.1.0-alpha.2/Cargo.toml --example basic --locked
cargo run --manifest-path target/package/warded-0.1.0-alpha.2/Cargo.toml --example server --locked -- --check
```

On PowerShell, set `$env:RUSTDOCFLAGS = "-D warnings"` before the documentation command. On a native
Windows developer machine, the MSVC C/C++ runtime libraries and Windows SDK must be installed and
discoverable; a missing `msvcrt.lib` prevents tests from linking before Warded runs.

The suite checks complete-condition fallthrough, overlap and declaration order, captures discarded
after failed alternatives and negation, collision errors, raw/repeated/invalid inputs, all required
matching dimensions, equivalent shortcut compositions, semantic negotiation, unread bodies and
trailers, committed service execution, no-match request preservation, readiness isolation, and an
Axum mounting boundary. A compile-fail doctest checks that matchers have no body accessor.

The [comparison harness](https://github.com/viell-dev/slop-warded-router/tree/main/research/comparison)
is an optional engineering probe,
independent of the library dependency graph. It records actual upstream behavior and negotiation
differences; it is not a benchmark or a required compatibility claim.

The server example additionally checks a real localhost HTTP/1.1 listener, host error conversion,
HEAD/405 behavior, known cleartext context, graceful shutdown, and Vary preservation. It does not
establish TLS/proxy trust, browser workflows, HTTP/2, or HTTP/3. Human use of a representative
candidate remains required before a stable release.

## Alpha.2 regression and cost probes

Six tests in `review_regressions.rs` were first run against unchanged alpha.1 (`f35a9ff`) in Linux
Docker, Rust 1.99.1: all failed for the reported media specificity/parameter ordering, lookup
priority/wildcards, concrete language definitions, and duplicate path captures. The same test bodies
remain as regression checks, alongside quoted/wildcard media cases, language exclusions, valid and
invalid tags, and field-byte limits with valid syntax and repeated-field separators.

The expanded suite checks borrowed configuration ownership, actual cross-thread selection,
constant-cost router cloning by service clone counts, copy-on-write isolation, boxed errors with
Axum bodies, non-default response types in request-returning mode, selected readiness failures,
cancellation, and outer concurrency admission. Shared-offer cache checks include differing sets,
heads, limits, and malformed-input policies. Strict operand-order behavior remains explicit.

Review regressions compare 50 escaped-name/catch-all combinations with matchit's decoded names,
reject malformed private-use tags while retaining valid private-use/grandfathered controls, and
exercise supported field-byte/member/parameter bounds across media, language, and encoding. The
unsupported token-limit setter was removed. Listener checks compare complete Vary tokens so
`accept-encoding` cannot satisfy an expected `accept` token.

Run `cargo bench --bench selection --locked` for the focused shared-offer cost probe: 1, 16, 64,
and 128 routes/offers; 257 valid Accept members; approximately 5.8 KiB; five warmups and 50 timed
selections per size; release profile; last selected target asserted every time. Construct a fresh
head per selection so the measurement includes cache creation. This is a diagnostic measurement,
not a CI timing threshold or an end-to-end throughput claim. Record hardware, compiler, limits,
and the exact commit when comparing runs.

## First alpha evidence

On 2026-11-10, the Linux Docker run with Rust 1.99.1 passed all commands above: 22 regression tests,
four doctests including the negative body-access check, each independent feature configuration,
Clippy with warnings denied, documentation, the example, and package reconstruction/execution.
The native Windows machine passed `cargo check`; its installed MSVC runtime libraries were
unavailable for linking. Hosted Windows CI supplies the required Windows execution evidence.

A negative control temporarily changed `Not` to accept a matching operand. The focused
`failed_branches_and_negation_cannot_publish_captures` test failed on its capture assertion, then
passed after the original source was restored. The mutation was made only in a disposable test
copy. The body-frame test also verifies that its poll counter advances when the returned body is
actually consumed, and the readiness fixture rejects calls on an unprepared clone.
