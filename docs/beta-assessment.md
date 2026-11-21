# First-release API and evidence assessment

The [agreed goal](https://github.com/viell-dev/slop-warded-router/issues/8) bounds the first stable
0.1 line. This assessment records the consumer-informed API decisions, measured costs and
dependency dispositions that inform promotion. The promotion PR records exact revisions,
executed checks, review findings and the gate decision.

## API decisions

Retain the alpha.3 production API. The representative application compiles against it without a
library feature change. The decisions below include the questions raised during scope review.

| Public surface | Freeze decision and evidence |
| --- | --- |
| `Head`, `TransportSecurity` | Keep the immutable, body-free borrowed view. Cloning starts a fresh negotiation cache. Only trusted transport context is exposed from extensions; arbitrary extension access is deliberately absent. Custom matchers can use method, URI, version and raw headers. Synthetic security/version checks do not prove real TLS or HTTP/2 hosting. |
| `Matcher`, `MatchResult`, `match_fn`, `try_match_fn` | Keep synchronous applicability and explicit miss/error separation. Function helpers require `Clone`; a caller needing non-cloneable local state can implement `Matcher`. Add no required trait methods in 0.1.x. Future optional methods still require a compatibility assessment, including downstream method ambiguity. |
| `And`, `Or`, `Not`, `Filter` | Keep left-to-right lazy semantics, error propagation and tentative-capture isolation. `filter` accepts a non-cloneable predicate locally; conditional traits determine shared/Tower use. The application uses an ordinary `UserId` parser and a separate username predicate without typed capture storage. |
| `Captures` | Keep owned raw strings, collision errors and read-only predicate access. The fixture's selected wrapper reads this extension; Leptos separately reads its parameter map. Typed handler values are parsed by the application. No implicit decoding or Axum `Path` population is promised. |
| `Selector`, `SyncSelector` | Keep local and shared tables distinct. Generic borrowed targets avoid imposing Tower, body or thread-safety requirements on local selection. Selection scans declarations in order. |
| `Selection`, `Selected` | Keep the public `index`, `captures`, `target` and `selection` fields. These are exhaustive structs: no new required field or changed field meaning in 0.1.x. A later richer metadata format needs a compatible separate API or 0.2.0; speculative extension fields are unnecessary now. |
| `DefinitionError`, `MatchError` | Keep displayable definition diagnostics and cloneable/equatable matcher errors. `MatchError` is non-exhaustive; `Custom(String)` does not retain a source error. Applications needing richer diagnostics can format context at the boundary. Changing that representation is deferred until a concrete consumer requirement justifies losing those traits or adding another API. |
| Raw matcher types and method/security helpers | Keep named `PathMatcher`, `ExactPath`, `HeaderMatcher`, `ParamMatcher`, `SecurityMatcher`, their existing constructors, and opaque primitive/composition return types. [#30](https://github.com/viell-dev/slop-warded-router/issues/30) renamed `QueryMatcher` and its `query_pair`/`query_present` constructors to `ParamMatcher`, `param_pair` and `param_present` before RC, so they are not confused with the `query()` method helper. Inference, `impl Matcher` or caller-owned boxing suffice for the tested use; helpers need not all have nameable concrete types. Any valid `http::Method`, including QUERY/extensions, is supported. |
| `Offers`, `Negotiated`, `Limits`, `LanguageMatching` | Keep acceptability distinct from preferred-offer selection, complete shared sets and configurable byte/member/parameter bounds. The two language modes remain exhaustive for 0.1.x. No token-length control or registry validation is promised. The parser dependency's types remain private. |
| `Router`, `RequestRouter` | Keep cloneable `Send + Sync` destinations with common response/error types. The fixture boxes ordinary Axum/Leptos services; its Send-only file/error handler is constructed per call from Sync configuration. This is a concrete integration cost, not evidence that arbitrary Send-only services are supported. Broader bounds wait for a consumer that cannot use this boundary. |
| Miss policies and dispatch errors | Keep the ordinary 404 adapter's `Default` body bound, including with a fallback. `Default` need not be empty; the host is responsible for its body semantics. `RequestRouter` supports non-default bodies and unread no-match requests. Non-exhaustive dispatch errors retain selected-service errors and their source chain. Tests cover custom bodies, boxed errors and fallback precedence. |
| Metadata extensions and nesting | Keep both `Selection` and separately accessible `Captures`, accepting one capture clone for handler ergonomics. Successful inner routing replaces these unscoped extensions; inner fallback retains outer metadata. URI/prefix rewriting and scoped nesting are outside 0.1. Preserve parent data in an application extension when needed. |
| Features/toolchain | Keep independent `tower` and `negotiation` features, both enabled by default; test all four combinations. Rust 1.99.1 is the minimum and required CI toolchain, and does not rise during the stable 0.1 line. |

No new API is required by the fixture. Keep the first stable scope router-only: no companion
server, automatic method responses, dedicated host matcher, typed query binder, universal adapter,
typed capture store, prefix nesting, or indexing implementation. These are explicit dispositions,
not accidental omissions. The release goal does not freeze internal algorithms.

## Consumer evidence and limits

[`integration/leptos`](../integration/leptos/README.md) contains a locked, independently buildable
workspace using the enclosing source. The beta.1 run recorded 21 Axum and 27 Warded passes, plus
3 expected failures for the broken ID predicate. These project totals included Node HTTP-client
cases repeated under each of the three browser project names. The maintained suite now counts the
HTTP-client checks once per mode and reports browser checks by engine.

Warded selects outer HTTP services. The app's own router handles rendering/extraction and hydrated
client navigation, whose lack of a new document request is verified. Full requests/reloads prove
HTTP selection. Gated streaming observes initial content before late content is released. The
fixture uses public framework handlers and explicitly supplies HEAD/405 policy. It does not claim
universal Leptos integration, all SSR modes, native Windows browsers, HTTP/2/3, TLS, proxy trust,
authentication, caches, or human usability. The separate Tower and HTTP/1.1 tests retain readiness,
cancellation and QUERY/custom-method evidence without duplicating them in the browser.

The deterministic [`input_corpus.rs`](../tests/input_corpus.rs) runs in ordinary required CI.
Seed `0x7761726465640001` covers 1024 raw-header/query/path cases, 1024 definition-boundary cases,
512 raw Accept-family cases across media, encoding and both language modes, and generated valid
member/parameter boundaries. Assertions check raw captures, repetition/presence, untouched encoded
values, error versus miss policy, field bounds including repeated-field separators, unique winners,
and cached versus fresh/cloned-head results. Panics fail with the input and seed. Seed
`0x7761726465640002` adds 256 grammar-guided valid fields for each of the four negotiation
configurations. Each is rendered with equivalent syntax variants, standalone and split across two
lines, and its preferred and acceptable offers come from a small structural oracle written from
RFC 9110, RFC 4647 and the documented rules, not from the selection code. Where those sources
leave the preferred offer or an offer's acceptability undecided, only that assertion is skipped and
counted; syntax and bound checks still run. Separate controls cover absent, empty and repeated
fields, including the 16-empty-element limit. This
supplements protocol examples; it is neither an exhaustive parser oracle nor coverage-guided
fuzzing.

## Selection-cost decision

Run `cargo bench --locked --bench selection`. Each timed sample asserts the selected target or
miss; path hits also assert the raw capture. Each selection creates a fresh `Head`, including its
cache. Three warmups precede 1000 path selections or 20 negotiation selections. The probe covers:

- Compiled path tables with 16, 128 and 1024 routes: early hit, last hit and miss.
- Independent media acceptability predicates versus a shared complete offer set: 16 and 128 routes,
  early/late/miss, four unsupported members or 1023, with a matching member appended for hits.
- Small fields of 239–257 bytes and near-bound fields of 61,379–61,397 bytes, within the default
  64 KiB and 1024-member limits.
- Near-64-KiB language lookup with three offers, repeated comparisons against a zero-quality
  exclusion and an asserted fallback. This row uses the same warmups, fresh head and 20 timed
  selections as the media cases.

The exact CSV records diagnostic means from Cargo's optimized benchmark profile. The beta.2
promotion PR records the refreshed CSV from the final candidate archive, including the source and
environment identifiers. It compares the added language row on the same input before and after
the linear-time exclusion check. The change bounds each exclusion check by the offer length.
Overall lookup cost still depends on the active ranges, positive truncations and offers. These
samples support the API decision below. The earlier alpha.2 shared-offer probe used a smaller
input and measured a different workload.

**Decision:** no indexing-driven API change is needed for the scoped first release. Linear path
selection has a documented cost; repeated negotiation dominates the measured adverse case. Keep
cheap applicable conditions before expensive helpers, lower field/member limits where appropriate,
share complete offers only when their applicability matches, and bound admission in the host.
The router has no automatic whole-table work budget. A large negotiation-first table with client
fields near the defaults is an explicitly expensive configuration.

Revisit indexing or shared parsing when a representative workload requires it. Compare maintained
router algorithms and hybrid designs, preserving declaration order across the complete predicate,
errors, raw URI/captures and non-path routes. Include small-table, construction and memory costs.
An index is not justified merely by a large route count, nor does this measurement prove that every
enterprise API meets its latency budget. No performance regression threshold is introduced.

## Beta.1 benchmark record

The [release checkpoint](https://github.com/viell-dev/slop-warded-router/issues/5#issuecomment-6785590604)
retains the final Git-archive run from `0897dddddb196ed801a1c6c8053e1bafb8d93df4`, whose tree matches
integrated beta.1 `59710b0`. Archive SHA-256
`07684a145ec43c91e2c85e342b61d61fe974b1bcb83c4532e5ee55b6eff03dc8`. This run used Rust 1.99.1
in Linux Docker on an Intel i7-4930K with four CPU quota, 4 GiB and 512 PIDs. Ordinary scheduler
activity was not isolated. The CSV shows mean microseconds per selection, including assertions.

```csv
strategy,routes,field_bytes,case,iterations,mean_us
path,16,0,early,1000,0.210
path,16,0,late,1000,0.382
path,16,0,miss,1000,0.226
path,128,0,early,1000,0.184
path,128,0,late,1000,1.764
path,128,0,miss,1000,1.707
path,1024,0,early,1000,0.184
path,1024,0,late,1000,15.374
path,1024,0,miss,1000,15.040
shared-offers,16,255,early,20,4.152
independent,16,255,early,20,2.385
shared-offers,16,256,late,20,6.350
independent,16,256,late,20,31.417
shared-offers,16,239,miss,20,5.632
independent,16,239,miss,20,28.169
shared-offers,16,61395,early,20,602.481
independent,16,61395,early,20,325.043
shared-offers,16,61396,late,20,604.549
independent,16,61396,late,20,5180.885
shared-offers,16,61379,miss,20,602.755
independent,16,61379,miss,20,5178.306
shared-offers,128,255,early,20,18.148
independent,128,255,early,20,1.867
shared-offers,128,257,late,20,35.405
independent,128,257,late,20,254.408
shared-offers,128,239,miss,20,32.648
independent,128,239,miss,20,224.531
shared-offers,128,61395,early,20,2672.373
independent,128,61395,early,20,323.301
shared-offers,128,61397,late,20,2686.992
independent,128,61397,late,20,37698.936
shared-offers,128,61379,miss,20,2351.841
independent,128,61379,miss,20,36311.075
```

## Dependency and follow-up dispositions

The [dependency overview](https://github.com/viell-dev/slop-warded-router/issues/4) tracks
maintained mitigations, upstream reports and reconsideration triggers. The
[upstream-test comparison](./upstream-tests.md) records the affected cases and pinned sources.

- Keep an exact parse-net pin, Warded scoring and retained protocol counterexamples. Beta.2 pins
  0.10.0 and reuses its media-range matching. Warded keeps local language rules and media precedence
  for the compatibility cases in the [design notes](./design.md). Retest updates with the
  counterexamples and corpus before relaxing the pin. Independent offer sets still parse
  independently, with the cost documented above.
- Keep oxilangtag syntax validation, private-use/grandfathered controls and the language-tags defect
  history. No reason is established to restore the replaced parser or require registry validation.
- Keep the 25-ordinary-parameter guard, unmatched-closer rejection and the original failed probes.
  Terminal catch-alls remain outside that capacity. Revisit only with tested dependency evidence.
- Beta.1 documented a one-byte ASCII-name workaround for matchit 0.9.2's rejection of some valid
  escaped suffixes. The local fix for [issue #13](https://github.com/viell-dev/slop-warded-router/issues/13)
  inserts a one-byte ASCII placeholder where matchit would shift the marker wrongly and returns
  the declared name. Retain the ASCII and Unicode matching regressions when reconsidering the
  fix for a future dependency version.
- Nesting, destination/body/clone bounds and definition diagnostics retain the API decisions above;
  change only for demonstrated consumer needs, without advertising broader support first.
- Contain the prototype cargo-leptos cleanup problem by starting/supervising the binary directly;
  verify failure cleanup before container teardown. Preserve the original observation; no upstream
  fix is required by this fixture. Remove the starter's ignored metadata from the maintained copy.
- Distribute through crates.io from `0.1.0-beta.3`, with docs.rs as the versioned API reference.
  GitHub private reporting, dependency policy checks and the hosted fixture gate are in place. The
  [release procedure](./release.md) describes exact-commit verification, publication and the
  owner's hands-on session before RC
  ([#26](https://github.com/viell-dev/slop-warded-router/issues/26)).

The recurring [upstream-test comparison](./upstream-tests.md) records affected areas and reuse for
unchanged ones. The [release procedure](./release.md) carries unfinished RC/stable work; beta
agreement does not waive it.
