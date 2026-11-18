# Representative Leptos application

This maintained fixture compares the same application with Axum HTTP routing and Warded HTTP
routing. It depends on the enclosing Warded source through `path = "../.."`; a checkout therefore
pins application evidence to the library being tested. It is excluded from the library package and
has an independent Cargo workspace and lockfile.

## Run

Use disposable Docker on Linux with four CPUs, 6 GiB RAM, 1 GiB shared memory and 1024 PIDs.
The Dockerfile pins the Playwright 1.65.0 base by digest, Rust 1.99.1 and cargo-leptos 0.3.12.
`.nvmrc` records the tested Node version. Save this as a Bash script and run it from the
repository root:

```bash
set -euo pipefail
docker build -t warded-leptos-test:rust1.99.1-playwright1.65.0 integration/leptos
evidence="$(mktemp -d)"
revision="$(git rev-parse HEAD)"
git archive --format=tar --output="$evidence/source.tar" "$revision"
sha256sum "$evidence/source.tar" | tee "$evidence/archive-sha256.txt"
container="$(docker create --interactive --cpus 4 --memory 6g --pids-limit 1024 --shm-size 1g \
  --env "WARDED_FIXTURE_REVISION=commit:$revision" \
  warded-leptos-test:rust1.99.1-playwright1.65.0 \
  bash -euc 'tar -xf - -C /app
    cd /app/integration/leptos
    exec timeout --kill-after=30s 15m bash check.sh')"
trap 'docker rm -f "$container" >/dev/null' EXIT
status=0
docker start --attach --interactive "$container" <"$evidence/source.tar" \
  2>&1 | tee "$evidence/fixture.log" || status=$?
echo "Fixture evidence: $evidence"
docker cp "$container:/app/integration/leptos/artifacts" "$evidence/"
exit "$status"
```

Docker and toolchain downloads are prerequisites. Each server binds an OS-assigned localhost port
inside the container. The runner queries `/__test/router` through the active router to confirm Axum,
Warded or the deliberately broken Warded configuration. Each JSON and HTML report records the
source marker, live router identity and URL. For an already provisioned Linux environment, run
`WARDED_FIXTURE_REVISION=commit:<sha> bash integration/leptos/check.sh` with the tested commit
SHA, or pass `archive-sha256:<sha256>` for a separately verified source archive.

The runner checks formatting, both compilation modes with Clippy, locked builds and TypeScript.
It then requires these Playwright results:

| Variant | Node HTTP client | Chromium | Firefox | WebKit | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| Axum | 3 passes | 5 passes | 5 passes | 5 passes | 18 passes |
| Warded | 5 passes | 6 passes | 6 passes | 6 passes | 23 passes |
| Broken ID predicate | 0 | 1 failure | 1 failure | 1 failure | 3 expected failures |

The negative control must fail specifically because `GET /users/42` returns 404 in place of 200.
Retries, skips, flakes, unexpected counts, missing reports and runner errors fail the command.
The runner supervises the compiled server directly and verifies that a child failing with exit 97
leaves a dead server PID and closed port while the supervisor is still in the container.

The ignored `artifacts/run.*/` directory holds each variant's JSON and HTML reports, server log,
identity and retained failure traces. Historically, beta.1 recorded 21 Axum passes, 27 Warded
passes and 3 broken-predicate failures. Those totals included Node HTTP-client cases repeated
under all three browser project names.

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
For the exact `/users/{value}` path shape, Warded's explicit rejection service returns a plain
404 for invalid, encoded or overflowing values. The HTTP client tests raw path bytes, which a
browser can normalize before sending. A trailing slash falls outside the exact shape.
In both modes, `/users/-1/` returns 404 with the Leptos user page and `-1` in its selected element.
This records Leptos's fallback rendering, whose route matching ignores the trailing slash; it is not
a Warded guarantee. An application that must not render rejected values should also route or
redirect trailing-slash variants, or validate in the component. A second resource tests a header
**and** a raw query condition, including ordered rejection and preserved URI.

After hydration, Leptos's browser router handles client navigation without a new document request.
The test verifies that boundary; it reloads to exercise the HTTP router again. Application
authorization and validation still belong at their appropriate server boundaries. A path predicate
is not a replacement for either.

## Triggers and limits

[Leptos fixture](../../.github/workflows/leptos-fixture.yml) runs on Ubuntu 24.04 for affected
pull requests and pushes to `main`, for unprefixed version tags, and via manual dispatch. On every
pull request a short job decides whether library source, the root manifest, Cargo configuration,
the toolchain file, the fixture (except its Markdown), the workflow or its path script changed.
The required `leptos-fixture` check needs a passing fixture run for an affected revision or a
skip with an explicit unaffected result. See the
[release procedure](../../docs/release.md#before-rc) for the candidate gates.
Hosted jobs build from the digest-pinned base and use fresh container storage. They stream the
source archive into the container and copy reports out as the host user before removing the
container. This keeps source and output off shared volumes and works with rootless Docker.
Available evidence is uploaded on success and failure and retained for 14 days. The image is rebuilt
from the Dockerfile for each hosted job. Package downloads during image and fixture setup are still
environment inputs; the Sass version that cargo-leptos downloads is pinned in the Dockerfile.

The fixture enables only Warded's `tower` feature. The browser environment is Linux over
localhost cleartext HTTP/1.1. This establishes one application,
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
