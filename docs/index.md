---
layout: home
description: "A Rust router that selects one destination for each HTTP request from ordered conditions on the whole request head."
hero:
  name: Warded
  text: Route on the whole request head
  tagline: A Rust router that selects one destination from ordered conditions on the method, path, headers, query, HTTP version and trusted HTTPS context.
  actions:
    - theme: brand
      text: Get started
      link: /getting-started
    - theme: alt
      text: Content negotiation
      link: /negotiation
    - theme: alt
      text: API reference
      link: https://docs.rs/warded/0.1.0-beta.5/warded/
features:
  - title: Every condition counts
    details: A route is selected only when all of its conditions hold. A route that fails on a header or a language lets selection continue, in the order you declared.
  - title: Negotiation as a condition
    details: Serve one path in several languages, as HTML or JSON, or precompressed, by routing on the client's Accept fields with explicit rules and a default.
  - title: Plain Tower services
    details: Dispatch to Axum handlers, Axum routers or any Tower service. Selection never reads the request body, and the selected service's response is final.
---

Serve a page in the language the client prefers, with English as the default:

```rust
use http::Request;
use warded::{
    Head, Matcher, Selector,
    matchers::{exact_path, get},
    negotiation::{LanguageMatching, Offers},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let languages = Offers::languages(["en", "fr"], LanguageMatching::Lookup)?;
    let welcome = || get().and(exact_path("/welcome"));
    let routes = Selector::new()
        .route(welcome().and(languages.selected("en")?), "Welcome")
        .route(welcome().and(languages.selected("fr")?), "Bienvenue")
        .route(welcome(), "Welcome");

    let request = Request::builder()
        .uri("/welcome")
        .header("accept-language", "fr-CA,fr;q=0.9,en;q=0.5")
        .body(())?;
    let selected = routes.select(&Head::from_request(&request))?;
    assert_eq!(selected.map(|selected| *selected.target), Some("Bienvenue"));
    Ok(())
}
```

[Get started](./getting-started.md) to install Warded and build a first route table, read [how
selection works](./selection.md), or go straight to [content negotiation](./negotiation.md). When
a route does not behave as you expect, see [troubleshooting](./troubleshooting.md). [Scope and
limits](./limits.md) lists what Warded leaves to your server and application, the tested
environments and what selection costs.
