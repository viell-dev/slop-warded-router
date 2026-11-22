---
description: "Route on the request method, raw header fields, raw query parameters and the HTTP version, and combine these conditions."
---

# Methods, headers and query parameters

Besides the path, a route can test the request method, raw header fields, raw query parameters and
the HTTP version. Each helper in `warded::matchers` tests one thing, and you combine them with
`and`, `or` and `not`. All of them compare raw values: nothing is decoded, trimmed or folded to
another case unless this page says so.

## Methods

`get()`, `head()`, `post()`, `put()`, `patch()`, `delete()`, `options()`, `connect()`, `trace()` and
`query()` each match one method and nothing else. `query()` is the QUERY method of [RFC
10008](https://www.rfc-editor.org/rfc/rfc10008), a safe request that carries its query in the
request content. For any other method, pass an `http::Method` to `method`:

```rust
use http::{Method, Request};
use warded::{
    Head, Matcher, Selector,
    matchers::{exact_path, get, head, method, post},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let propfind = Method::from_bytes(b"PROPFIND")?;
    let routes = Selector::new()
        .route(get().or(head()).and(exact_path("/orders")), "list orders")
        .route(post().and(exact_path("/orders")), "create order")
        .route(
            method(propfind).and(exact_path("/orders")),
            "describe orders",
        )
        // Any other method on this path: answer 405 Method Not Allowed yourself.
        .route(exact_path("/orders"), "method not allowed");

    let select = |method: &str| -> Result<_, Box<dyn std::error::Error>> {
        let request = Request::builder().method(method).uri("/orders").body(())?;
        let selected = routes.select(&Head::from_request(&request))?;
        Ok(selected.map(|selected| *selected.target))
    };
    assert_eq!(select("GET")?, Some("list orders"));
    assert_eq!(select("HEAD")?, Some("list orders"));
    assert_eq!(select("PROPFIND")?, Some("describe orders"));
    // Methods are case-sensitive: "propfind" is a different extension method.
    assert_eq!(select("propfind")?, Some("method not allowed"));
    assert_eq!(select("DELETE")?, Some("method not allowed"));
    Ok(())
}
```

Things Warded leaves to you or your HTTP stack:

- **`get()` does not match HEAD.** Use `get().or(head())` to serve both. The server still applies
  the rule that a HEAD response has no body.
- **No automatic 405, `Allow` or OPTIONS responses.** Declare them as routes, as above, or mount a
  framework destination that provides them: an Axum `MethodRouter` answers 405 with `Allow` for its
  own path. See [Tower and Axum](./tower.md#mounting-axum-routers).
- **Matchers never read the content.** A QUERY or POST route is chosen from the head; the
  selected service reads the body.

Two shortcuts combine a method with [transport security](./transport-security.md):
`secured_post()` is `secured().and(post())` and `secured_get_and_head()` is
`secured().and(get().or(head()))`. They behave exactly like the longer forms.

## Headers

Three helpers test one header field by name. Names are case-insensitive; values are compared as
exact bytes:

| Helper                       | Matches when                                                     |
| ---------------------------- | ---------------------------------------------------------------- |
| `header_eq(name, value)`     | The field occurs exactly once, with exactly this value           |
| `header_any_eq(name, value)` | At least one occurrence of the field has exactly this value      |
| `header_present(name)`       | The field occurs at least once, with any value                   |

A request can repeat a field. `header_eq` misses when the field is repeated, even if every copy
has the expected value, so a request cannot satisfy it with a second, conflicting copy. Use
`header_any_eq` when repetition is legitimate. Values do not need to be UTF-8:
`header_eq("x-raw", [0xff])` compares one byte.

```rust
use http::Request;
use warded::{
    Head, Matcher,
    matchers::{header_any_eq, header_eq, header_present},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request = Request::builder()
        .header("x-feature", "beta")
        .header("x-feature", "dark-mode")
        .header("x-tenant", "acme")
        .body(())?;
    let head = Head::from_request(&request);

    assert!(header_eq("X-Tenant", "acme")?.evaluate(&head)?.is_some());
    assert!(header_eq("x-tenant", "ACME")?.evaluate(&head)?.is_none());
    // Repeated: header_eq misses, header_any_eq finds the value.
    assert!(header_eq("x-feature", "beta")?.evaluate(&head)?.is_none());
    assert!(
        header_any_eq("x-feature", "beta")?
            .evaluate(&head)?
            .is_some()
    );
    assert!(header_present("x-feature")?.evaluate(&head)?.is_some());

    // Invalid names and values fail when the route is defined.
    assert!(header_present("not a name").is_err());
    assert!(header_eq("x-tenant", "line\nbreak").is_err());
    Ok(())
}
```

Do not compare `Accept`, `Accept-Language` or `Accept-Encoding` with `header_eq`. Those fields are
weighted lists: a browser sends `fr-CA,fr;q=0.9,en;q=0.8`, which is not equal to `fr` even though
the client accepts French. [Content negotiation](./negotiation.md) reads them properly.

## Query parameters

`param_pair(key, value)` and `param_present(key)` test the raw query string of the URI. The string
is split on `&`, and each piece at its first `=`:

- `param_pair("format", "csv")` matches a piece that is exactly `format=csv`.
- `param_present("debug")` matches `debug` on its own or `debug=` with any value.
- A repeated key matches if any of its pieces matches; the order of pieces does not matter.
- Keys and values are compared as written: `+` is not a space, `%63sv` is not `csv`, and `CSV` is
  not `csv`.
- A bare `flag` is not the same as `flag=`: `param_pair("flag", "")` matches only `flag=`.

```rust
use http::Request;
use warded::{
    Head, Matcher, Selector,
    matchers::{exact_path, get, param_pair, param_present},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let search = || get().and(exact_path("/search"));
    let routes = Selector::new()
        .route(search().and(param_pair("format", "csv")), "csv export")
        .route(search().and(param_present("debug")), "debug view")
        .route(search(), "results page");

    let select = |uri: &str| -> Result<_, Box<dyn std::error::Error>> {
        let request = Request::builder().uri(uri).body(())?;
        let selected = routes.select(&Head::from_request(&request))?;
        Ok(selected.map(|selected| *selected.target))
    };
    assert_eq!(select("/search?q=rust&format=csv")?, Some("csv export"));
    assert_eq!(
        select("/search?format=json&format=csv")?,
        Some("csv export")
    );
    assert_eq!(select("/search?q=rust&debug")?, Some("debug view"));
    // Raw comparison: neither of these is format=csv.
    assert_eq!(select("/search?format=CSV")?, Some("results page"));
    assert_eq!(select("/search?format=%63sv")?, Some("results page"));
    Ok(())
}
```

These helpers route on the presence or exact spelling of a parameter. Parse and validate query
values in the selected handler, for example with your framework's query extractor.

`param_*` reads the URI's query string. The `query()` method helper is unrelated: it matches
requests whose method is QUERY.

## HTTP version

`version(http::Version::HTTP_2)` matches requests the server received over that HTTP version. It
needs no path, so it can route a whole protocol version. Behind a reverse proxy it describes the
connection from the proxy to your application, not the client's connection.

## Combining conditions

Conditions combine freely, and a route may use any of them without a path. Helpers that take a
string, such as `exact_path`, `header_eq` and `param_pair`, copy it, so you can build routes from
configuration read at startup. [How selection
works](./selection.md#combinators-and-evaluation-order) explains evaluation order, and [custom
matchers](./custom-matchers.md) cover conditions these helpers do not, such as the request host.
