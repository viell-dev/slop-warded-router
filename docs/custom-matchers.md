---
description: "Write your own route conditions with match_fn, try_match_fn or the Matcher trait, and the rules they must follow."
---

# Custom matchers

When the built-in conditions do not express a rule, write your own. A matcher reads the request
head, the method, URI, version, raw headers and transport security, and returns a match, a miss or
an error. It cannot read the body or arbitrary extensions. Custom matchers combine with built-in
ones like any other condition.

There are three ways to write one:

| Tool           | Use it when                                                  |
| -------------- | ------------------------------------------------------------ |
| `match_fn`     | A yes-or-no rule without captures or errors                  |
| `try_match_fn` | The rule can fail, or should produce captures                |
| `impl Matcher` | The matcher holds state that is not `Clone`, or you want a named type |

## A host condition with `match_fn`

Warded has no host helper, because finding the request's host depends on the protocol. HTTP/1.1
puts it in the `Host` field; HTTP/2 and absolute-form requests put it in the URI's authority, which
then takes precedence. This matcher checks the URI first and uses `Host` only when the URI has no
authority:

```rust
use http::{Request, Version, header::HOST, uri::Authority};
use warded::{Head, Matcher, Selector, match_fn, matchers::exact_path};

/// Whether the request is for this host name, ignoring a port and ASCII case.
fn host_is(head: &Head<'_>, expected: &str) -> bool {
    if let Some(authority) = head.uri().authority() {
        return authority.host().eq_ignore_ascii_case(expected);
    }
    // Without an authority in the URI, HTTP/1.1 requires exactly one Host field.
    let mut fields = head.headers().get_all(HOST).iter();
    match (fields.next(), fields.next()) {
        (Some(field), None) => field
            .to_str()
            .ok()
            .and_then(|value| value.parse::<Authority>().ok())
            .is_some_and(|authority| authority.host().eq_ignore_ascii_case(expected)),
        _ => false,
    }
}

fn host(expected: &'static str) -> impl Matcher + Clone {
    match_fn(move |head| host_is(head, expected))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let routes = Selector::new()
        .route(
            host("api.example.com").and(exact_path("/status")),
            "API status",
        )
        .route(exact_path("/status"), "site status");

    let select = |request: Request<()>| -> Result<_, Box<dyn std::error::Error>> {
        let selected = routes.select(&Head::from_request(&request))?;
        Ok(selected.map(|selected| *selected.target))
    };

    // HTTP/1.1 origin form: the host is in the Host field.
    let request = Request::builder()
        .uri("/status")
        .header("host", "API.example.com:8443")
        .body(())?;
    assert_eq!(select(request)?, Some("API status"));

    // HTTP/2: the server puts the :authority in the URI.
    let request = Request::builder()
        .version(Version::HTTP_2)
        .uri("https://api.example.com/status")
        .body(())?;
    assert_eq!(select(request)?, Some("API status"));

    // A Host field does not override the URI's authority.
    let request = Request::builder()
        .uri("https://www.example.com/status")
        .header("host", "api.example.com")
        .body(())?;
    assert_eq!(select(request)?, Some("site status"));
    Ok(())
}
```

Adapt the comparison to your deployment: a reverse proxy may rewrite the host, and some
applications compare the port too. A raw host condition, like a raw path, is not an access control
by itself.

`match_fn` needs a closure that is `Clone`. It produces no captures and cannot fail. Its result is
`Send` and `Sync` when the closure is, so it works in a `Router`.

## Errors and captures with `try_match_fn`

`try_match_fn` takes a closure returning a `MatchResult`:

- `Ok(Some(captures))` matches, with captures for the selected route, which may be empty;
- `Ok(None)` misses;
- `Err(error)` stops selection.

Return an error only when continuing could select the wrong route. A field that is simply absent
is a miss; a field that is present but unusable may deserve an error. This matcher captures a
tenant name and rejects one it cannot read:

```rust
use http::Request;
use warded::{Captures, Head, MatchError, Matcher, try_match_fn};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tenant = try_match_fn(|head| {
        let Some(value) = head.headers().get("x-tenant") else {
            return Ok(None);
        };
        let name = value
            .to_str()
            .map_err(|_| MatchError::Custom("x-tenant is not visible ASCII".into()))?;
        let mut captures = Captures::default();
        captures.insert("tenant", name)?;
        Ok(Some(captures))
    });

    let request = Request::builder().header("x-tenant", "acme").body(())?;
    let captures = tenant
        .evaluate(&Head::from_request(&request))?
        .expect("a tenant");
    assert_eq!(captures.get("tenant"), Some("acme"));

    let request = Request::builder().body(())?;
    assert!(tenant.evaluate(&Head::from_request(&request))?.is_none());

    let request = Request::builder()
        .header("x-tenant", &b"\xff"[..])
        .body(())?;
    assert!(matches!(
        tenant.evaluate(&Head::from_request(&request)),
        Err(MatchError::Custom(_))
    ));
    Ok(())
}
```

`Captures::insert` reports `DuplicateCapture` instead of overwriting a name, and the same happens
when two matchers combined with `and` capture the same name. Pick names that do not collide with
your path parameters.

## Implementing `Matcher`

A type that implements `Matcher` needs only `evaluate`. Use this for state that is expensive or
impossible to clone, such as a large set loaded from configuration:

```rust
use http::{Request, header::HOST};
use std::collections::BTreeSet;
use warded::{Captures, Head, MatchResult, Matcher, SyncSelector};

/// Matches requests whose Host field names one of the configured hosts.
struct AllowedHosts(BTreeSet<String>);

impl Matcher for AllowedHosts {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        let host = head
            .headers()
            .get(HOST)
            .and_then(|value| value.to_str().ok())
            .map(|value| {
                value
                    .split(':')
                    .next()
                    .unwrap_or(value)
                    .to_ascii_lowercase()
            });
        Ok(host
            .is_some_and(|host| self.0.contains(&host))
            .then(Captures::default))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hosts = AllowedHosts(["example.com", "www.example.com"].map(String::from).into());
    let routes = SyncSelector::new().route(hosts.and(warded::matchers::get()), "site");

    let request = Request::builder()
        .header("host", "WWW.example.com")
        .body(())?;
    let selected = routes.select(&Head::from_request(&request))?;
    assert_eq!(selected.map(|selected| *selected.target), Some("site"));
    Ok(())
}
```

The trait's provided methods `and`, `or`, `not` and `filter` work on your type too. This version
is simpler than the `match_fn` one above and suits only HTTP/1.1 origin-form requests; it shows
the trait, not a complete host rule.

## Rules for custom matchers

- **Keep them fast and bounded.** A matcher runs for every request that reaches its route. Avoid
  I/O, locks held across calls and work that grows with client input without a limit.
- **Keep them free of side effects.** A matcher may run for requests that end up on another route,
  or not run at all when an earlier condition already missed. Count or log in the selected
  service instead.
- **Thread safety follows your state.** `Selector` accepts matchers that use `Rc` or `Cell`.
  `SyncSelector` and `Router` need `Send + Sync` matchers.
- **Read only the head.** The body is not available, by design: routing never consumes or waits
  for request content.
