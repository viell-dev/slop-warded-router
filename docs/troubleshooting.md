# Troubleshooting

This page covers the two kinds of problems you are likely to meet: a route that cannot be defined,
and a request that does not reach the route you expected.

## Definition errors

Constructors that parse something, such as `path`, `header_eq` and `Offers::languages`, return
`Result<_, DefinitionError>`. Build the route table at startup and treat an error as a bug in the
table. The message names the problem:

| Message                                                                 | Cause and fix                                                                    |
| ----------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `a path pattern must start with / and contain no query or fragment`    | Start the pattern with `/`. Match query parameters with `param_pair` instead.     |
| `"/users/{id": Parameters must be registered with a valid name`          | Close the brace and give the parameter a name. The quoted pattern comes first.    |
| `unmatched closing brace in path pattern`                               | Remove the `}`, or write `}}` for a literal brace.                                |
| `capture name "id:[0-9]+" contains ':' or whitespace; …`               | Write `{id}` and check the value with `filter`; see [other routers](#path-syntax-from-other-routers). |
| `duplicate capture name: id`                                            | Rename one parameter; a catch-all counts too.                                     |
| `"…": Catch-all parameters are only allowed at the end of a route`      | Move `{*name}` to the end, or use an ordinary parameter.                          |
| `"…": Only one parameter is allowed per path segment`                   | Split the segment, or capture it whole and check it with `filter`.                |
| `path patterns support at most 25 ordinary named parameters`            | Capture the rest with a final `{*rest}` and parse it yourself.                    |
| `invalid HTTP header name` or `failed to parse header value`            | Header names are tokens such as `x-tenant`; values cannot contain line breaks.    |
| `an offer set must not be empty`                                         | Give `Offers` at least one offer.                                                 |
| `invalid or duplicate server offer for accept-language: "en_US"`        | Use a well-formed tag (`en-US`), once. Languages and codings compare without case. |
| `invalid or duplicate server offer for accept: "*/*"`                   | Offer concrete media types, without wildcards or `q` weights.                     |
| `offer is not registered: "fr"`                                          | `selected` takes an offer exactly as you listed it in the set.                    |

## Errors while selecting

`select`, and a `Router`, stop with a `MatchError` when a condition cannot be evaluated:

- **`trusted HTTPS context is missing`** (`MissingSecurityContext`): a `secured()` or `unsecure()`
  condition ran, but nothing inserted a `TransportSecurity` value. Add the layer that supplies it,
  move the condition after the path so it only runs where needed, or use `unknown_as_miss()`. See
  [transport security](./transport-security.md#unknown-context).
- **`invalid accept-language: …`** (`InvalidHeader`): a negotiation condition with
  `invalid_is_error` read a malformed field. That is the client's error; map it to 400. Remove
  `invalid_is_error` if malformed fields should fall through to a later route instead.
- **`duplicate capture name: …`** (`DuplicateCapture`): two conditions combined with `and`
  captured the same name. Rename one.
- **Your own message** (`Custom`): a custom matcher returned an error.

## A request does not reach the expected route

Start by asking Warded. `select` on a synthetic request shows the selected route's declaration
index, and `evaluate` on a single condition shows which one fails:

```rust
use http::Request;
use warded::{
    Head, Matcher, Selector,
    matchers::{exact_path, get, header_eq},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let routes = Selector::new()
        .route(get().and(exact_path("/orders/")), "orders")
        .route(get(), "catch-all");

    let request = Request::builder()
        .uri("/orders")
        .header("x-mode", "Preview")
        .body(())?;
    let head = Head::from_request(&request);

    let selected = routes.select(&head)?.expect("the catch-all matches");
    assert_eq!(selected.selection.index, 1);

    // Test the first route's conditions one at a time.
    assert!(get().evaluate(&head)?.is_some());
    assert!(exact_path("/orders/").evaluate(&head)?.is_none()); // the trailing slash
    assert!(header_eq("x-mode", "preview")?.evaluate(&head)?.is_none()); // the case
    Ok(())
}
```

Common causes, roughly in order of frequency:

- **An earlier route matches first.** Order is the only priority; a broad route declared early,
  such as a pathless one, hides later routes. Move specific routes up.
- **The path is not exactly the pattern.** A trailing slash, a different case or a percent escape
  makes a different path. Warded does not normalize or redirect. See [raw
  paths](./paths.md#raw-paths).
- **The pattern uses another router's syntax.** See [path syntax from other
  routers](#path-syntax-from-other-routers) below.
- **HEAD with a `get()` route.** `get()` matches only GET; use `get().or(head())`.
- **A repeated header with `header_eq`.** `header_eq` needs exactly one field; use
  `header_any_eq` when the field may repeat.
- **Header values differ in case or spacing.** Values compare as exact bytes.
- **Query values are encoded or use `+`.** `param_pair` compares the raw text; `%20` and `+` are
  not decoded.
- **The wrong language matching mode.** With lookup, a request for `fr` does not reach an offer
  `fr-CA`; with basic filtering, a request for `fr-CA` does not reach `fr`. See [lookup or basic
  filtering](./negotiation.md#lookup-or-basic-filtering).
- **An empty or malformed `Accept` or `Accept-Language` field.** It accepts no offer, so the
  negotiated routes miss. Add a final default route.
- **A shared offer set across different conditions.** The set may prefer an offer whose route
  fails for this request. See [shared offers](./negotiation.md#shared-offers-or-independent-conditions).
- **Unknown transport security with `unknown_as_miss`.** The route misses silently; check that the
  server inserts `TransportSecurity`.
- **A selected service answered 404.** The router never tries later routes after selecting one.
  With a mounted Axum router, the request may reach the right application and then miss inside it.

## Path syntax from other routers

Warded declares parameters only with braces, and a parameter takes no type or pattern. [Syntax
from other routers](./paths.md#syntax-from-other-routers) explains the rules with an example.

- **`/articles/:slug` never matches `/articles/hello`.** Outside braces, `:` and `*` are literal
  text, so the route matches only a request for `/articles/:slug`. Write `/articles/{slug}`, and
  `/files/{*rest}` instead of `/files/*rest`.
- **`{id:[0-9]+}` or `{id:int}` is a definition error.** A name cannot contain `:` or whitespace,
  so constraint syntax is refused when you build the route. Write `{id}` and check the value with
  [`filter`](./paths.md#refine-captures-with-filter). 0.1.0-beta.4 and earlier accepted such a
  pattern, with the whole text as the name, and it matched any value; see
  [upgrading](./upgrading.md#from-0-1-0-beta-4).
- **`{ id }` is a definition error.** Remove the spaces inside the braces.

## The response is not what the route should produce

- **A cache served another variant.** Add `Vary` for every negotiated field; see [caching and
  Vary](./negotiation.md#caching-and-vary).
- **An Axum handler answers 500 `No paths parameters found for matched route`.** The handler
  uses Axum's `Path` extractor, which reads Axum's own route parameters. Warded inserts its
  captures as the `Captures` and `Selection` request extensions instead, so read them with
  `Extension<Captures>`, as in the example below. A fallback receives no captures, and a nested
  router's match replaces the outer ones; see [nested routers](./tower.md#nested-routers).
- **The body was never read.** That is the selected service's job; Warded passes it through
  unread.

The same route with each extractor:

```rust
use axum::{
    Extension,
    body::Body,
    extract::Path,
    http::{Request, StatusCode},
    routing::get,
};
use http_body_util::BodyExt;
use tower::ServiceExt;
use warded::{Captures, Router, matchers::path};

/// Axum's extractor: it finds no parameters, because Axum did not match this route.
async fn with_path(Path(id): Path<String>) -> String {
    format!("order {id}")
}

/// Warded's captures, from the request extensions.
async fn with_captures(Extension(captures): Extension<Captures>) -> String {
    format!("order {}", captures.get("id").unwrap_or_default())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request = || Request::builder().uri("/orders/42").body(Body::empty());

    let mistaken = Router::new().route(path("/orders/{id}")?, get(with_path));
    let response = mistaken.oneshot(request()?).await?;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response.into_body().collect().await?.to_bytes(),
        "No paths parameters found for matched route"
    );

    let fixed = Router::new().route(path("/orders/{id}")?, get(with_captures));
    let response = fixed.oneshot(request()?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.into_body().collect().await?.to_bytes(), "order 42");
    Ok(())
}
```

A mounted `axum::Router` matches its own routes, so `Path` works inside it; see [mounting Axum
routers](./tower.md#mounting-axum-routers).

## Reporting a problem

If Warded behaves differently from this documentation, open a [GitHub
issue](https://github.com/viell-dev/slop-warded-router/issues) with the Warded version, Rust
version, enabled features, a synthetic request and route table, the result you expected and the
result you got. Report suspected vulnerabilities privately through the [security
policy](https://github.com/viell-dev/slop-warded-router/security/policy) instead.
