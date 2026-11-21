# How selection works

A route table is a list. For each request, Warded evaluates the routes from first to last and
selects the first one whose conditions all hold. This page explains that rule and its
consequences: why order matters, what happens when a condition fails, and how combinators decide
which conditions run.

## The first complete match wins

Every condition on a route takes part in the decision before anything is selected. A route that
matches the path but not a header is not selected, and selection continues with the next route.
Conditions on the method, path, headers, query, HTTP version and transport security are equal
peers. A route does not need a path condition at all.

```rust
use http::{Request, Version};
use warded::{
    Head, Matcher, Selector,
    matchers::{exact_path, header_eq, version},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let routes = Selector::new()
        .route(
            exact_path("/report").and(header_eq("x-preview", "on")?),
            "preview report",
        )
        .route(exact_path("/report"), "report")
        // No path condition: every HTTP/1.0 request that reaches this route matches it.
        .route(version(Version::HTTP_10), "legacy client");

    let select = |request: Request<()>| -> Result<_, Box<dyn std::error::Error>> {
        let selected = routes.select(&Head::from_request(&request))?;
        Ok(selected.map(|selected| *selected.target))
    };

    // The preview header satisfies the first route.
    let request = Request::builder()
        .uri("/report")
        .header("x-preview", "on")
        .body(())?;
    assert_eq!(select(request)?, Some("preview report"));

    // Without it, the first route fails on the header and selection continues.
    let request = Request::builder().uri("/report").body(())?;
    assert_eq!(select(request)?, Some("report"));

    // An HTTP/1.0 request for /report matches the second route first.
    let request = Request::builder()
        .uri("/report")
        .version(Version::HTTP_10)
        .body(())?;
    assert_eq!(select(request)?, Some("report"));

    // Any other HTTP/1.0 request reaches the pathless route.
    let request = Request::builder()
        .uri("/anything")
        .version(Version::HTTP_10)
        .body(())?;
    assert_eq!(select(request)?, Some("legacy client"));
    Ok(())
}
```

Order is the only priority. Warded does not rank routes by how specific their paths are, so a
broad route declared early hides narrower routes declared after it. Put specific routes first and
general ones, such as a default for a path, last.

## What a selection contains

`select` returns a `Selected` value with two parts:

- `target`: a reference to the value you registered with the route.
- `selection`: a `Selection` with the zero-based `index` of the route in declaration order and the
  `captures` of that route only. Captures from routes that were tried and failed are discarded.

A `Router` puts the same `Selection`, and a separate copy of its `Captures`, into the request's
extensions before calling the selected service.

## Misses and errors

A condition has three possible results:

- **Match.** The condition holds. A path pattern also produces captures.
- **Miss.** The condition does not hold. Selection moves on to the next route.
- **Error.** The condition could not be evaluated. Selection stops and `select` returns the
  error; later routes are not tried.

An error is never treated as a miss, because guessing could select a route the request should not
reach. Errors are `MatchError` values from a few places:

| Error                    | Cause                                                                |
| ------------------------ | -------------------------------------------------------------------- |
| `MissingSecurityContext` | `secured()` or `unsecure()` ran without [transport security](./transport-security.md) information |
| `InvalidHeader`          | A negotiation condition set to [`invalid_is_error`](./negotiation.md#absent-empty-and-malformed-headers) read a malformed header |
| `DuplicateCapture`       | Two conditions on one route captured the same name                   |
| `Custom`                 | A [custom matcher](./custom-matchers.md) reported a failure           |

When every route misses, `Selector` returns `None` and `Router` responds 404, or calls its fallback.
See [Tower and Axum](./tower.md#misses-and-fallbacks).

## Combinators and evaluation order

Matchers combine with three methods from the `Matcher` trait:

- `a.and(b)` holds when both hold. It evaluates `a` first and evaluates `b` only if `a` matched.
- `a.or(b)` holds when either holds. It evaluates `b` only if `a` missed.
- `a.not()` holds when `a` misses. It never produces captures.

A fourth method, `filter`, adds a condition on a matcher's captures; see [refine
captures](./paths.md#refine-captures-with-filter).

Evaluation runs left to right and stops as soon as the result is known. An error from any
evaluated condition stops the whole selection, including inside `or` and `not`: `not` turns a match
into a miss and a miss into a match, but it passes an error through.

Because a condition that is never evaluated cannot fail, the order of an `and` decides where an
error can happen. A strict condition placed after a path only runs for requests on that path:

```rust
use http::Request;
use warded::{
    Head, MatchError, Matcher, Selector,
    matchers::{exact_path, secured},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The request carries no transport security information.
    let request = Request::builder().uri("/").body(())?;
    let head = Head::from_request(&request);

    // Checking security first fails for every request, even one for another path.
    let strict_first = Selector::new()
        .route(secured().and(exact_path("/admin")), "admin")
        .route(exact_path("/"), "home");
    assert_eq!(
        strict_first.select(&head).err(),
        Some(MatchError::MissingSecurityContext)
    );

    // Checking the path first misses /admin and continues to the home page.
    let path_first = Selector::new()
        .route(exact_path("/admin").and(secured()), "admin")
        .route(exact_path("/"), "home");
    let selected = path_first.select(&head)?.expect("the home route matches");
    assert_eq!(*selected.target, "home");
    Ok(())
}
```

Ordering also helps performance. Put cheap conditions, such as the method and path, before
expensive ones such as negotiation, so most routes are rejected before a header is parsed.

## Captures in combinations

Each matcher's captures are tentative until its route is selected:

- `and` keeps the captures of both sides, and reports `DuplicateCapture` if both captured the same
  name.
- `or` keeps the captures of whichever side matched; a side that failed contributes nothing.
- `not` and a failed `filter` discard captures.

A later route never sees captures from an earlier route that failed.

## Selector, SyncSelector and Router

All three apply the same rules:

| Type           | Targets          | Thread safety                         | Needs feature |
| -------------- | ---------------- | ------------------------------------- | ------------- |
| `Selector`     | Any value        | None required; matchers may hold `Rc` | No            |
| `SyncSelector` | Any value        | Matchers must be `Send + Sync`        | No            |
| `Router`       | Tower services   | Matchers and services `Send + Sync`   | `tower`       |

Selection scans the routes in order on each request, so its cost grows with the number of routes
that are tried before one matches. See [scope and limits](./limits.md#performance) for measurements
and advice.
