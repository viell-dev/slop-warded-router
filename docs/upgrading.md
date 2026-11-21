# Upgrading

This page lists the releases whose upgrade needs changes to your code, newest first. Each section
says what changed and what to do. A release that is not listed needs nothing beyond the new
version, and the [changelog](./changelog.md) lists every change.

To upgrade, change the exact version in `Cargo.toml` and run `cargo update --workspace`. When you
skip releases, apply the section of each listed release between your version and the new one,
oldest first.

## From 0.1.0-beta.3

The raw query-string helpers are renamed, so they can no longer be confused with the new `query()`
helper for the QUERY method. Their matching is unchanged, and the old names are gone:

| 0.1.0-beta.3                  | Now                           |
| ----------------------------- | ----------------------------- |
| `matchers::query_pair`        | `matchers::param_pair`        |
| `matchers::query_present`     | `matchers::param_present`     |
| `matchers::QueryMatcher`      | `matchers::ParamMatcher`      |

```diff
-use warded::matchers::{exact_path, query_pair, query_present};
+use warded::matchers::{exact_path, param_pair, param_present};

-let export = exact_path("/search").and(query_pair("format", "csv"));
-let debug = exact_path("/search").and(query_present("debug"));
+let export = exact_path("/search").and(param_pair("format", "csv"));
+let debug = exact_path("/search").and(param_present("debug"));
```

The compiler reports every remaining use of the old names. `matchers::query()` is new and matches
the QUERY method; see [methods](./conditions.md#methods).

Encoding offer sets also choose differently, without a compile error, when `Accept-Encoding`
lists neither `identity` nor `*`. An `identity` offer then has no weight and is selected only
when none of the listed codings you offer is acceptable; 0.1.0-beta.3 gave it weight 1. With
the offers `br`, `gzip` and `identity`, `br;q=0.5, gzip;q=0.8` now selects `gzip`. Such requests
reach a compressed route instead of the `identity` one, so check that it sets `Content-Encoding`
and `Vary: Accept-Encoding`; see [precompressed files](./negotiation.md#serve-precompressed-files).
