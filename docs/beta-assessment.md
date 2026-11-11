# First-release API and evidence assessment

The [agreed goal](https://github.com/viell-dev/slop-warded-router/issues/8) bounds the first stable
0.1 line. This is the consumer-informed freeze assessment for the beta.1 source candidate, not
an RC/stable approval or a claim of independent implementation review. The promotion PR records
exact revisions, executed checks, review findings and the final gate decision.

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
| Raw matcher types and method/security helpers | Keep named `PathMatcher`, `ExactPath`, `HeaderMatcher`, `QueryMatcher`, `SecurityMatcher`, their existing constructors, and opaque primitive/composition return types. Inference, `impl Matcher` or caller-owned boxing suffice for the tested use; helpers need not all have nameable concrete types. Any valid `http::Method`, including QUERY/extensions, is supported. |
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
workspace using the enclosing source. Seven common cases run in each of three browser engines
against Axum and Warded; Warded adds two selection cases per engine. Reports require 21 and 27
passes respectively, no retries/skips/flakes, plus three expected assertion failures when the ID
predicate is deliberately broken. The runner also exercises cleanup while a child fails with an
active server. Missing reports or runner errors cannot count as passing evidence.

Warded selects outer HTTP services. The app's own router handles rendering/extraction and hydrated
client navigation, whose lack of a new document request is verified. Full requests/reloads prove
HTTP selection. Gated streaming observes initial content before late content is released. The
fixture uses public framework handlers and explicitly supplies HEAD/405 policy. It does not claim
universal Leptos integration, all SSR modes, native Windows browsers, HTTP/2/3, TLS, proxy trust,
authentication, caches, or human usability. The separate Tower and HTTP/1.1 tests retain readiness,
cancellation and QUERY/custom-method evidence without duplicating them in the browser.

The deterministic [`input_corpus.rs`](../tests/input_corpus.rs) runs in ordinary required CI.
Seed `0x7761726465640001` covers 1024 raw-header/query/path cases, 1024 definition-boundary cases,
512 Accept-family cases across media, encoding and both language modes, and generated valid
member/parameter boundaries. Assertions check raw captures, repetition/presence, untouched encoded
values, error versus miss policy, field bounds including repeated-field separators, unique winners,
and cached versus fresh/cloned-head results. Panics fail with the input and seed. This supplements
protocol examples; it is neither an exhaustive parser oracle nor coverage-guided fuzzing.

## Selection-cost decision

Run `cargo bench --bench selection --locked`. Each timed sample asserts the selected target or
miss; path hits also assert the raw capture. Each selection creates a fresh `Head`, including its
cache. Three warmups precede 1000 path selections or 20 negotiation selections. The probe covers:

- Compiled path tables with 16, 128 and 1024 routes: early hit, last hit and miss.
- Independent media acceptability predicates versus a shared complete offer set: 16 and 128 routes,
  early/late/miss, four unsupported members or 1023, with a matching member appended for hits.
- Approximately 240–257 bytes and 61,379–61,397 bytes, within the default 64 KiB/1024-member limits.

The development and candidate measurements used Linux Docker, Rust 1.99.1 release profile, an
Intel i7-4930K host, four CPU quota, 4 GiB and 512 PIDs. They measured about 0.16–0.20 µs for early
path hits, 13.6–17.1 µs for 1024-route last hits/misses, and about 37–42 ms for 128 independent
negotiation-first late hits/misses near the field bound. Shared offers reduced the latter shape to
about 2.3–2.7 ms, while costing more than an independent early hit because the whole offer set is
scored. These are mean diagnostic samples on one host, including assertions, not percentiles or a
portable throughput promise. The host was not isolated; the candidate run overlapped browser work.
The promotion PR retains the final candidate run and its exact revision; do not compare these
shapes directly with the earlier smaller shared-offer probe as if only the implementation changed.

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

## Dependency and follow-up dispositions

The [dependency overview](https://github.com/viell-dev/slop-warded-router/issues/4) stays open for
maintained mitigations. During this preparation, both existing upstream reports remained open with
no comments; no acceptance of further agent-authored submissions is inferred and no new upstream
report was submitted. matchit's latest listed tag remains 0.9.2. No dependency upgrade is bundled.

- Keep the exact parse-net 0.9.3 pin, Warded scoring and retained protocol counterexamples. Retest a
  proposed update before relaxing the pin or removing the workaround. Independent offer sets still
  parse independently; the cost is documented above. The unsupported token control remains absent.
- Keep oxilangtag syntax validation, private-use/grandfathered controls and the language-tags defect
  history. No reason is established to restore the replaced parser or require registry validation.
- Keep the 25-ordinary-parameter guard, unmatched-closer rejection and the original failed probes.
  Terminal catch-alls remain outside that capacity. Revisit only with tested dependency evidence.
- [Issue #13](https://github.com/viell-dev/slop-warded-router/issues/13): document the inherited valid
  escaped-suffix rejection. The verified `/{a}{{x}}` workaround has a positive matching control.
  This rare configuration limitation is accepted for 0.1; revisit if the workaround fails or a
  consumer cannot use it. It is not an uncontained panic or a newly established routing vulnerability.
- Nesting, destination/body/clone bounds and definition diagnostics retain the API decisions above;
  change only for demonstrated consumer needs, without advertising broader support first.
- Contain the prototype cargo-leptos cleanup problem by starting/supervising the binary directly;
  verify failure cleanup before container teardown. Preserve the original observation; no upstream
  fix is required by this fixture. Remove the starter's ignored metadata from the maintained copy.
- Keep publication disabled. Private reporting, current advisory/license/installation preparation,
  a hosted fixture workflow, audience/docs channel and human use remain at their agreed RC/stable
  stages. This candidate starts none of those distribution channels.

The recurring [upstream-test comparison](./upstream-tests.md) records affected areas and reuse for
unchanged ones. The [release procedure](./release.md) carries unfinished RC/stable work; beta
agreement does not waive it.
