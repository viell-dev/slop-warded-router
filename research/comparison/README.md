# Upstream behavior comparison

This independent, non-publishable probe package records the selection and fallback behavior that
informed Warded's first alpha. It is not part of the published library or its required unit tests.
Direct comparison versions and the complete resolution are pinned in `Cargo.toml` and `Cargo.lock`.

Run it with Rust 1.99.1 in a disposable Docker container:

```sh
docker run --rm --cpus 4 --memory 4g --pids-limit 512 \
  -v "$PWD:/work" -w /work rust:1.99.1 cargo run --locked
```

The recorded [results](./results.txt) passed on Linux in the official Rust 1.99.1 image on
2026-11-10. These are synthetic in-process request checks, not network protocol, performance, or
universal compatibility results. All five head cases run against Actix, Warp, Salvo, and Rama.
Actix additionally exercises same-path alternatives and static/parameter overlap with resource
guards. Negotiation probes cover `parse-net` media preference, parameter matching, exclusions,
strict malformed-quality handling, repeated fields, language schemes, and encoding defaults.

The complete-predicate model is already possible in existing routers. Warded's focused selector
keeps an immutable body-free interface, a distinct operational error channel, transactional
captures under negation, and an ordinary `http`/Tower boundary. Adopting an entire Actix, Warp,
Salvo, or Rama stack would make its framework or service contracts the required boundary. Rama's
generic core remains useful precedent, but its boolean result does not directly express the
separate evaluation errors needed here. The small Warded combinators are original implementations;
no upstream source has been copied.

`parse-net` is selected as the ordinary dependency for negotiation: it provides the needed
field-specific parser and selection operations with no dependencies and an explicit malformed
policy. Warded configures refusal in the parser and maps the result to its chosen miss/error policy;
it does not inherit an HTTP status response from the dependency. `headers-accept` accepts invalid
quality values through a default, and `negotiator` has different absent-encoding behavior. Those are
observed differences from the desired helper contract, not claims of undocumented upstream defects.

`matchit` is reused once per path pattern, so its most-specific-path lookup cannot suppress another
candidate. Route declaration order remains authoritative across complete predicates. This alpha
uses a linear scan and makes no speed claim.
