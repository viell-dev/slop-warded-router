---
description: "What Warded does, what your server and application provide, which environments are tested, and what selection costs."
---

# Scope and limits

Warded is a router: it selects one destination for a request from ordered conditions on the
request head, and can dispatch to Tower services. Everything around that selection belongs to your
server, framework or application. This page lists where that boundary is, which environments are
tested, and what selection costs.

## What Warded does

- Evaluates routes in declaration order and selects the first whose conditions all hold, across
  method, path, headers, query parameters, HTTP version and trusted transport security.
- Keeps misses and errors apart: a failed condition moves on, an unevaluable one stops.
- Captures raw path parameters, and refines them with your predicates.
- Negotiates `Accept`, `Accept-Language` and `Accept-Encoding` to choose between your offers.
- Dispatches to one Tower service per request, with a 404, a fallback or the unmatched request
  when nothing matches.
- Never reads, buffers or waits for the request body.

## What your application provides

| Concern                                           | Where it belongs                                        |
| ------------------------------------------------- | ------------------------------------------------------- |
| Listening, TLS, HTTP/2 and HTTP/3                 | The server, such as Hyper through Axum                  |
| Whether a request arrived over HTTPS              | The listener or proxy layer that inserts `TransportSecurity` |
| Authentication and authorization                  | The destination or middleware; HTTPS is not identity     |
| 405 with `Allow`, OPTIONS, HEAD for GET, CORS     | Explicit routes, or a framework destination such as an Axum `MethodRouter` |
| Decoding, normalizing or redirecting paths        | The destination, after selection                         |
| Host-based routing                                | A [custom matcher](./custom-matchers.md#a-host-condition-with-match-fn) |
| Typed path and query values                       | The destination's parsing or extractors                 |
| Prefix stripping for nested applications          | The destination; [nested routers](./tower.md#nested-routers) match full paths |
| `Vary`, `Content-*` headers, compression          | Layers and destinations; see [negotiation](./negotiation.md#what-your-application-decides) |
| Status codes for routing errors                   | Your [error mapping](./tower.md#map-errors-to-responses) |
| Timeouts, retries, concurrency limits             | Tower middleware around the router                      |

## Tested environments

- **Library:** the test suite runs in CI on Windows and Linux with Rust 1.99.1, in all four
  feature combinations, together with the doctests, the examples on this site, the repository
  examples and the packaged crate.
- **Hosting:** real HTTP/1.1 over localhost, through the [server example] and a representative
  Leptos server-rendered application that uses Warded as its outer router, tested with Chromium,
  Firefox and WebKit on Linux.
- **Synthetic only:** HTTP/2 and HTTP/3 requests, TLS and reverse proxies. The `version` and
  transport security conditions are tested with constructed requests, not with real servers or
  proxies of those kinds.

Report results from other environments, good or bad, in a [GitHub
issue](https://github.com/viell-dev/slop-warded-router/issues).

## Performance

Selection tries routes one after another, so its cost grows with the number of routes evaluated
before a match, and with what each condition does. There is no index over paths, and Warded makes
no throughput promise.

The project's measurements, on one desktop machine in a container, give a sense of scale:

- **Paths:** with compiled patterns, a hit on the first route takes about 0.2 µs. A hit on the
  last of 1024 routes, or a miss, takes about 15 µs.
- **Negotiation:** a shared offer set parses the field once per request. Independent
  `accepts_*` conditions parse it once each, so 128 of them over a 61 KB `Accept` field took about
  37 ms for a late hit, against about 2.7 ms for one shared set.

These figures are diagnostic, not guarantees, and the [measurement record] describes the inputs
and environment. To keep selection cheap:

- Put cheap conditions such as method and path before negotiation and custom matchers.
- Share one offer set among alternative routes instead of many independent conditions.
- Lower the [negotiation limits](./negotiation.md#limits-and-cost) where clients send short fields.
- Limit concurrent requests in the server, since the router admits everything.

[server example]: https://github.com/viell-dev/slop-warded-router/blob/0.1.0-beta.5/examples/server.rs
[measurement record]: https://github.com/viell-dev/slop-warded-router/blob/0.1.0-beta.5/internals/beta-assessment.md#selection-cost-decision
