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
cargo package --locked
cargo run --manifest-path target/package/warded-0.1.0-alpha.1/Cargo.toml --example basic --locked
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

Synthetic request and Tower checks do not prove listener behavior, TLS trust configuration, browser
workflows, or protocol negotiation. Human use of a representative candidate remains required before
a stable release; these engineering checks do not supply that evidence.

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
