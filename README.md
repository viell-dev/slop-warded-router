# Path constraint experiment

> [!NOTE]
> Agent-authored research submitted on viell-dev's behalf by GPT-6.1 Sol via Codex. Not reviewed by
> the owner.

This independent root branch prototypes value-constrained path matching against Warded alpha.2.
`Refine::filter` is an experimental consumer extension, not an implemented Warded API. It refines a
successful match using a boolean predicate over tentative captures. Newtypes are optional: a
predicate may call `FromStr` without changing the capture representation.

Compare an ordinary predicate, the same validation through a `UserId` newtype, and a precompiled
regex candidate with the same numeric range check. All candidates must produce equal selected
routes, raw captures, and handler-side values. Regex alone does not reject u64 overflow, so its
candidate includes the semantic range check rather than benchmarking a different contract.

The fixture policies are decimal u64 identifiers and ASCII usernames beginning with a letter.
They are experiment inputs, not required application grammars. Tests also check overlap under a
more permissive username predicate: declaration order decides when both conditions accept a value.
The existing raw URI semantics, percent encoding, errors, and capture isolation remain intact.

## Decision

Recommend a capture-refinement operation taking an ordinary predicate, with the prototype spelling
`matcher.filter(|captures| ...)`. Keep it separate from the path-pattern grammar and preserve
Warded's existing first-complete-match order. Newtypes, free functions, closures, and precompiled
regexes can all be used inside that callback. No newtype trait, typed capture storage, regex
dependency, or typed query API is required by this capability.

```rust
use warded::matchers::path;
use warded_path_constraint_probe::Refine; // This experiment's extension, not a Warded import.

let ids = path("/users/{id}")?.filter(|captures| {
    captures.get("id").is_some_and(|raw| raw.parse::<u64>().is_ok())
});
```

Expected rejection is a miss. The callback runs only after a successful underlying match; errors
still propagate. A successful callback preserves the raw captures. Values from rejected candidates
are discarded. If two conditions accept the same input, declaration order decides. Do not turn an
example's ID or username grammar into a library default.

The whole-capture callback can express multi-parameter constraints. It cannot statically validate
literal capture names used inside a closure; a caller must handle missing values explicitly.
Typed result retention and named-parameter convenience methods remain possible follow-ups when
consumer evidence justifies them. This probe does not measure a typed capture cache.

## Reproduce

From this checkout on a Linux Docker host:

```sh
docker run --rm --cpus 4 --memory 4g --pids-limit 512 \
  -v "$PWD:/app" -v warded-path-target:/app/target \
  -v warded-path-registry:/usr/local/cargo/registry \
  -v warded-path-git:/usr/local/cargo/git -w /app \
  rust:1.99.1 bash -c 'bash scripts/check.sh && bash scripts/measure.sh'
```

Correctness: five focused tests pass with all features and without default features, including
10,000 seeded inputs compared across the candidates. Controls cover selected targets and raw
values, u64 overflow, missing/encoded/invalid values, method and header conditions, overlapping
predicates, suffixes/catch-alls, capture rollback, and operational-error propagation. Clippy with
warnings denied and formatting pass. Disabling predicate rejection in a disposable copy makes the
capture-isolation test fail; restoring the source returns the checks to green.

Timing uses Rust 1.99.1 release optimization and the normal system allocator: three processes,
nine rotated samples per process, 100,000 operations/sample, and 5,000 warmup operations/scenario.
Construction samples each build and drop 100 three-route tables. Container limits are four CPUs,
4 GiB RAM, and 512 PIDs on Linux x86_64. ID-hit timings include a second parse in the selected
handler for every candidate, keeping their output and work comparable.

Allocation counting is a separate build with a forwarding global allocator and 10,000 operations
per case. Its allocation/reallocation calls and requested byte counts are not live/peak memory or
timing results. The driver refuses timing mode with that allocator enabled. Instrumentation's
small unsafe allocator wrapper is confined to this experimental binary and delegates to System.

See [raw process measurements](./results/timing-1.csv), [second](./results/timing-2.csv),
[third](./results/timing-3.csv), [median summary](./results/summary.csv),
[allocation audit](./results/allocations.csv), and [comparison](./results/comparison.html).
Summary points are medians of three process medians; the bounds span those medians, not a
confidence interval. The scripts generate these CSVs using an invariant numeric locale.

The predicate and newtype-backed predicate have identical allocation counts in every measured
case; they call the same underlying validation function. Short-ID process medians overlap. Regex
is faster for the invalid-input miss and slower for several other cases, while compiling its
two expressions increases table build/drop cost. These findings favor a flexible callback as the
core API, rather than prescribing a type or a pattern language. A long-lived server may easily
amortize regex compilation; this is not a reason to prohibit consumer-supplied regexes.

These are exploratory measurements of one three-route fixture, not timing gates, large-API
scaling evidence, full framework integration, or production performance promises. QUERY and query
parameter mapping are outside this probe; no typed query binding is introduced.

Original experiment code is MIT OR Apache-2.0. Dependencies retain their own licenses. Cargo
publication is disabled.
