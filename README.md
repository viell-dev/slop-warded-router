# Warded

A Rust router that selects a service using the complete declared conditions on the request head:
method, path, headers, query, HTTP version, and trusted HTTPS context. A route need not constrain
the path. Selection and service execution are separate.

**Status:** first-alpha development. There is no published crate or release yet. APIs and behavior
may change. This repository contains the router; it does not implement an HTTP server or TLS stack.

Agents write and maintain Warded. No person reviews the code. See
[CONTRIBUTING.md](./CONTRIBUTING.md) for contribution and verification expectations.

Intended licensing: MIT OR Apache-2.0, at the recipient's option. See
[LICENSE-MIT](./LICENSE-MIT) and [LICENSE-APACHE](./LICENSE-APACHE).
