# Paths and captures

Warded has two path conditions. `exact_path` compares the whole path with one string. `path`
compiles a pattern with named parameters, such as `/users/{id}`, and captures the parts of the path
they match. Both compare the path exactly as it arrived, without decoding or cleaning it.

## Exact paths

`exact_path("/orders")` matches the path `/orders` and nothing else: not `/orders/`, not `/Orders`
and not `/orders%2F`. The query string is not part of the path, so `/orders?page=2` matches too.
It accepts any string, including one read from configuration.

## Patterns and captures

A pattern is a path with parameters in braces:

- `{name}` matches one non-empty segment, or part of one, up to the next `/`. A segment can
  hold one parameter with literal text around it, as in `/media/{name}.png`.
- `{*name}` is a catch-all that matches the non-empty rest of the path, including `/`. It must end
  the pattern.
- `{{` and `}}` stand for literal `{` and `}`.

The selected route's captures are in `selection.captures`, and a `Router` also adds them to the
request's extensions as `Captures`:

```rust
use http::Request;
use std::error::Error;
use warded::{
    Captures, Head, Matcher, Selector,
    matchers::{get, path},
};

/// Select a route for a GET request and return its target with its captures.
fn select(
    routes: &Selector<&'static str>,
    uri: &str,
) -> Result<Option<(&'static str, Captures)>, Box<dyn Error>> {
    let request = Request::builder().uri(uri).body(())?;
    let selected = routes.select(&Head::from_request(&request))?;
    Ok(selected.map(|selected| (*selected.target, selected.selection.captures)))
}

fn main() -> Result<(), Box<dyn Error>> {
    let routes = Selector::new()
        .route(get().and(path("/users/{id}")?), "user")
        .route(get().and(path("/users/{id}/posts/{post}")?), "post")
        .route(get().and(path("/media/{name}.png")?), "image")
        .route(get().and(path("/files/{*rest}")?), "file");

    let (target, captures) = select(&routes, "/users/42/posts/7")?.expect("a route matches");
    assert_eq!(target, "post");
    assert_eq!(captures.get("id"), Some("42"));
    assert_eq!(captures.get("post"), Some("7"));

    let (target, captures) = select(&routes, "/media/logo.png")?.expect("a route matches");
    assert_eq!(target, "image");
    assert_eq!(captures.get("name"), Some("logo"));

    let (target, captures) = select(&routes, "/files/css/site.css")?.expect("a route matches");
    assert_eq!(target, "file");
    assert_eq!(captures.get("rest"), Some("css/site.css"));

    // Parameters never match an empty segment, and the trailing slash matters.
    for uri in ["/users/", "/users/42/", "/files/", "/media/.png"] {
        assert!(select(&routes, uri)?.is_none(), "{uri}");
    }
    Ok(())
}
```

Captures are owned strings in name order. Read one with `captures.get("id")`, which returns
`Option<&str>`, or iterate with `captures.iter()`.

## Raw paths

Warded matches the path the HTTP stack delivered, byte for byte:

- Percent escapes stay encoded. `/users/caf%C3%A9` captures `caf%C3%A9`, and `/a%2Fb` is one
  segment, not two.
- Letter case is significant: `/Users/42` does not match `/users/{id}`.
- Dot segments and repeated slashes are not removed: `/users/../admin` is not `/admin`.
- Nothing redirects. A request for `/orders/` does not reach `/orders`; declare both if you want
  both to work.

```rust
use http::Request;
use warded::{
    Head, Matcher,
    matchers::{exact_path, path},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user = path("/users/{id}")?;
    let request = Request::builder().uri("/users/caf%C3%A9").body(())?;
    let captures = user
        .evaluate(&Head::from_request(&request))?
        .expect("one encoded segment matches");
    assert_eq!(captures.get("id"), Some("caf%C3%A9"));

    for uri in ["/Users/42", "/users/a/b", "/users/../admin"] {
        let request = Request::builder().uri(uri).body(())?;
        assert!(
            user.evaluate(&Head::from_request(&request))?.is_none(),
            "{uri}"
        );
    }

    let admin = exact_path("/admin");
    let request = Request::builder().uri("/%61dmin").body(())?;
    assert!(admin.evaluate(&Head::from_request(&request))?.is_none());
    Ok(())
}
```

Decode captured values in your handler if you need text, and decide there how to treat an encoded
`/`. Because a destination may decode or normalize the path differently, raw path conditions are
not an access control. A route that excludes `/admin` does not exclude `/%61dmin`, which a
destination may decode to `/admin`; check authorization in the destination.

## Refine captures with `filter`

Two routes can share a pattern and differ in what the captured value looks like. `filter` adds a
condition on a matcher's captures. It runs only after the matcher succeeds; returning `false` is an
ordinary miss, so selection continues with the next route:

```rust
use http::Request;
use std::str::FromStr;
use warded::{
    Head, Matcher, Selector,
    matchers::{get, path},
};

/// A user id: one or more ASCII digits that fit in a u64.
struct UserId(u64);

impl FromStr for UserId {
    type Err = ();

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(());
        }
        raw.parse().map(Self).map_err(|_| ())
    }
}

fn is_username(raw: &str) -> bool {
    let mut bytes = raw.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let routes = Selector::new()
        .route(
            get().and(path("/users/{id}")?.filter(|captures| {
                captures
                    .get("id")
                    .is_some_and(|raw| raw.parse::<UserId>().is_ok())
            })),
            "user by id",
        )
        .route(
            get().and(
                path("/users/{name}")?
                    .filter(|captures| captures.get("name").is_some_and(is_username)),
            ),
            "user by name",
        );

    let select = |uri: &str| -> Result<_, Box<dyn std::error::Error>> {
        let request = Request::builder().uri(uri).body(())?;
        let selected = routes.select(&Head::from_request(&request))?;
        Ok(selected.map(|selected| *selected.target))
    };
    assert_eq!(select("/users/42")?, Some("user by id"));
    assert_eq!(select("/users/alice_7")?, Some("user by name"));
    // Neither: too large for a u64, and not a username either.
    assert_eq!(select("/users/18446744073709551616")?, None);
    assert_eq!(select("/users/-1")?, None);
    Ok(())
}
```

These rules are your application's policy; Warded gives no route priority to numbers or any other
kind of value. The rules for `filter`:

- The predicate receives the raw captured strings. A parsed value is not kept, so parse it again
  in the handler, or keep the parsing in one function as `UserId` does here.
- A predicate sees only the captures of the matcher it wraps. To compare captures from several
  matchers, call `filter` on their combination, such as `a.and(b).filter(...)`.
- `captures.get` returns `None` for a name the matcher does not capture. Decide what that means,
  as `is_some_and` does above by rejecting it.
- If the wrapped matcher returns an error, the predicate does not run and the error propagates.
- A predicate can use a precompiled regular expression or any other bounded check. Keep it free of
  side effects: it runs for every request that reaches the route.

## Pattern rules and limits

Patterns are checked when you call `path`, which returns a `DefinitionError` for:

- a pattern that does not start with `/`, or that contains `?` or `#`;
- an unclosed `{` or an unmatched `}`;
- two parameters with the same name, including a catch-all;
- a catch-all that is not at the end of the pattern;
- more than one parameter in a segment, such as `/{from}-{to}`;
- more than 25 ordinary parameters. A final catch-all does not count toward the limit.

```rust
use warded::matchers::path;

fn main() {
    for invalid in [
        "users/{id}",
        "/search?q={term}",
        "/users/{id",
        "/users/id}",
        "/users/{id}/{*id}",
        "/files/{*rest}/edit",
        "/{from}-{to}",
    ] {
        assert!(path(invalid).is_err(), "{invalid}");
    }
    // Doubled braces are literals, also directly after a parameter.
    assert!(path("/templates/{{name}}").is_ok());
    assert!(path("/{id}{{x}}").is_ok());
}
```

Each pattern is compiled separately and tried in declaration order like any other condition, so a
literal route does not automatically win over a parameter route; whichever is declared first and
matches is selected. [Troubleshooting](./troubleshooting.md#definition-errors) lists the error
messages.
