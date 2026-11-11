# Representative Leptos application

This maintained fixture compares the same application with Axum HTTP routing and Warded HTTP
routing. It depends on the enclosing Warded source through `path = "../.."`; a checkout therefore
pins application evidence to the library being tested. It is excluded from the library package and
has an independent Cargo workspace and lockfile.

## Run

Use disposable Docker on Linux with four CPUs, 6 GiB RAM, 1 GiB shared memory and 1024 PIDs.
The image pins Rust 1.99.1, cargo-leptos 0.3.12 and Playwright 1.65.0; `.nvmrc` records its tested
Node version. Mount the **repository root** at `/app`:

```sh
docker build -t warded-leptos-test:rust1.99.1-playwright1.65.0 integration/leptos
docker run --rm --cpus 4 --memory 6g --pids-limit 1024 --shm-size 1g \
  -v "$PWD:/app" -w /app/integration/leptos \
  warded-leptos-test:rust1.99.1-playwright1.65.0 timeout 600s bash check.sh
```

Run both commands from the repository root. Docker, its image cache and toolchain downloads are
prerequisites; no host port or application account is needed. Optional named Cargo, build, npm and
tool-download caches can reduce repeated build cost. Do not share a writable build directory with
a simultaneous run. For an already provisioned Linux environment, `bash integration/leptos/check.sh`
is the equivalent entry point. Native Windows browser execution has not been established.

The runner checks formatting, both compilation modes with Clippy, locked builds, TypeScript and:

- **Axum:** seven cases in each of Chromium, Firefox and WebKit: exactly 21 passes.
- **Warded:** the same seven plus two selection cases in each browser: exactly 27 passes.
- **Negative control:** the fixture's numeric-ID predicate deliberately rejects IDs; exactly three
  selected checks must fail with 404 instead of 200. A runner crash or empty selection is not an
  acceptable negative result.
- **Failure cleanup:** a supervised child exits 97 while its server is alive. Before the container
  exits, the supervisor requires that exit code, a dead server PID and a closed listening port.

Retries, skips, flakes, unexpected counts, missing JSON reports and runner errors fail the command.
Reports and server logs go to ignored `artifacts/`; failing Playwright traces go to
`end2end/test-results/`. Preserve these before removing a disposable export. The script starts
the compiled binary itself, so cargo-leptos's end-to-end process cleanup does not own its lifetime.

## What this demonstrates

Warded is the outer HTTP router in its variant. The selected services use public Leptos SSR,
server-function and file/error handlers. A startup assertion limits the supported route list to
the four tested paths, GET and out-of-order SSR; this is not a general generated-route adapter.

The common cases exercise SSR HTML, CSS/favicon, hydration and an interactive counter; rendered
404, HEAD and explicit 405 policy; dynamic page requests; a hydrated client navigation followed by
a full reload; a real `ActionForm` POST with punctuation and Unicode; a server-function error; a
selected 500 without reselection; and a gated SSR stream. The streaming check observes initial
HTML **before** releasing the late resource, then verifies late content and hydration. It has no
latency threshold.

The Warded-only cases split `/users/{value}` between a `UserId` parser and a username predicate.
The selected wrapper reads Warded's `Captures` extension and returns the chosen kind and exact raw
capture in response headers. Leptos independently reads its own parameter map for rendering.
Invalid, encoded and overflowing IDs cannot re-enter through the framework fallback. A second
resource tests a header **and** a raw query condition, including ordered rejection and preserved URI.

After hydration, Leptos's browser router handles client navigation without a new document request.
The test verifies that boundary; it reloads to exercise the HTTP router again. Application
authorization and validation still belong at their appropriate server boundaries. A path predicate
is not a replacement for either.

## Triggers and limits

Run this command for changes to this fixture, matching/captures/selection, dispatch/readiness/body
handling, dependency/toolchain changes affecting those surfaces, and every beta/RC/stable candidate.
Retain exact-commit or verified-identical-tree evidence. Ordinary deterministic library checks stay
in required Windows/Linux CI. Before RC, add a maintained hosted workflow with manual,
affected-change and release triggers, bounded execution and retained artifacts; this manual command
supplies the beta-stage gate, as agreed in [issue #8](https://github.com/viell-dev/slop-warded-router/issues/8).

The browser environment is Linux over localhost cleartext HTTP/1.1. This establishes one application,
not all Leptos modes/extractors or framework-wide compatibility. Tower readiness and cancellation,
and QUERY/custom-method body preservation, have separate library/HTTP checks. HTTP/2, HTTP/3,
real TLS/proxy trust, authentication, cache behavior and the companion server remain outside this
fixture. The test-only release/failure endpoints and mutation switch belong only to this application.

## Provenance

Adapted from `leptos-rs/start-axum` at
[`6de10e4`](https://github.com/leptos-rs/start-axum/tree/6de10e4e89c80fea5121a95ed163a2b7da1b51dd),
through the repository's isolated `experiment-leptos` branch. The starter's Unlicense and README are
retained as `UPSTREAM-LICENSE` and `UPSTREAM-README.md`, along with its styles and favicon. Warded's
adaptations and new tests use the enclosing repository's MIT OR Apache-2.0 license. The pinned
framework/dependency versions and their own license terms remain in the lockfiles.
