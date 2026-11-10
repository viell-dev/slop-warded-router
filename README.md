# Leptos router experiment

> [!NOTE]
> Agent-authored experiment submitted on viell's behalf by GPT-6.1 Sol via Codex. Not reviewed by
> viell.

An isolated experiment on Warded's `experiment-leptos` orphan branch. This is not part of the
library package or a released application.

The [baseline commit](https://github.com/viell-dev/slop-warded-router/commit/56526d73c177436521edb9ef5c44ed4c061b5f81)
is the official Leptos–Axum starter at
[`6de10e4`](https://github.com/leptos-rs/start-axum/tree/6de10e4e89c80fea5121a95ed163a2b7da1b51dd),
instantiated as `warded-leptos-probe` without nightly features. Its application and handlers are
unchanged. The root commit retains Axum dispatch; the current version replaces HTTP dispatch with
Warded while retaining Leptos, the client router, assets, rendering, and hydration.

Reproduction uses Rust 1.99.1, cargo-leptos 0.3.12, locked Cargo dependencies, and Playwright 1.65.0.
The Dockerfile supplies the Rust/WASM tooling and browser dependencies in a disposable environment.
Native Node commands use the version in `.nvmrc`. Test execution disables retries and uses one
worker so failures remain visible.

From this branch's checkout, using a Linux shell and Docker:

```sh
docker build -t warded-leptos-test .
docker run --rm --cpus 4 --memory 6g --pids-limit 1024 --shm-size 1g \
  -v "$PWD:/app" \
  -v warded-leptos-target:/app/target \
  -v warded-leptos-registry:/usr/local/cargo/registry \
  -v warded-leptos-git:/usr/local/cargo/git \
  -v warded-leptos-node:/app/end2end/node_modules \
  -v warded-leptos-cache:/root/.cache \
  warded-leptos-test bash scripts/check.sh
```

The server and browser run together inside the disposable container; no host port is published.
Named volumes cache build dependencies. The script checks that all nine Playwright cases completed,
with zero skips, flaky results, or retries. A missing result file is a failure.

On 2026-11-10 the unchanged application with Axum dispatch passed all nine checks across Chromium,
Firefox, and WebKit on Linux Docker, Rust 1.99.1, Node 26.13.0, and Playwright 1.65.0. Checks cover
SSR without JavaScript, working hydrated counter interaction, CSS and favicon assets, rendered 404,
HEAD, and POST refusal with GET/HEAD Allow tokens. Tooling normalization and external tests are
experiment additions; application behavior remains the starter's.

Warded at [`0819b31`](https://github.com/viell-dev/slop-warded-router/commit/0819b312f2358b4808baca6364d3c034d8f21dcd)
is pinned as a Git dependency with only its Tower feature. The same nine checks also pass with this
replacement. `src/main.rs` uses Warded directly, `Handler::with_state` for Leptos's public rendering
and fallback handlers, and `tower::make::Shared` for the existing Axum listener. There is no Axum
HTTP router underneath Warded. HEAD suppression and the root resource's 405/Allow response are
explicit compatibility policies in the host adapter.

The adapter asserts that Leptos generated the one expected GET UI route with its expected SSR mode.
It fails startup if that generated route list changes. The client-side Leptos router remains intact;
replacing it is a different experiment. Server-function registration is outside this adapter.

After a deliberate mutation making the SSR predicate always miss, all nine checks failed with HTTP
405 and the command exited 1. Restoring the route returned all nine checks to green. The final
source also passed these checks in the same Docker environment:

```sh
cargo clippy --locked --no-default-features --features ssr --all-targets -- -D warnings
rustfmt --check --edition 2021 src/main.rs
```

The failing-test control also produced a cargo-leptos 0.3.12 `ProcessHandle` cleanup panic after
Playwright had written its nine failures. The exit remained nonzero and Docker removed the test
container. Keep this tooling limitation visible if converting the prototype into a maintained CI
fixture; the container boundary currently owns final process cleanup. The starter's ignored `env`
metadata warning is inherited and occurs in both variants.

This one-route starter has no server functions. Dynamic paths, other SSR modes, nested layouts,
server functions, authentication, production deployment, native Windows execution, and HTTP/2–3
remain untested. Passing browser engines in Docker is not a human usability or release sign-off.

Original template provenance and instructions are retained in [UPSTREAM-README.md](./UPSTREAM-README.md)
and [UPSTREAM-LICENSE](./UPSTREAM-LICENSE). The upstream template is Unlicense. Original experiment
adaptations are MIT OR Apache-2.0; dependency licenses remain their own. Cargo publication is disabled.
