# Leptos router experiment

An isolated experiment on Warded's `experiment-leptos` orphan branch. This is not part of the
library package or a released application. Agents maintain this fixture; no person has reviewed it.

The baseline application is the official Leptos–Axum starter at
[`6de10e4`](https://github.com/leptos-rs/start-axum/tree/6de10e4e89c80fea5121a95ed163a2b7da1b51dd),
instantiated as `warded-leptos-probe` without nightly features. Its application and handlers are
initially unchanged. The experiment will compare its Axum dispatch with Warded while retaining
Leptos, the client router, assets, rendering, and hydration.

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

This one-route starter has no server functions. Dynamic paths, other SSR modes, nested layouts,
server functions, authentication, production deployment, native Windows execution, and HTTP/2–3
remain untested. Passing browser engines in Docker is not a human usability or release sign-off.

Original template provenance and instructions are retained in [UPSTREAM-README.md](./UPSTREAM-README.md)
and [UPSTREAM-LICENSE](./UPSTREAM-LICENSE). The upstream template is Unlicense. Original experiment
adaptations are MIT OR Apache-2.0; dependency licenses remain their own. Cargo publication is disabled.
