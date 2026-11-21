# Warded

A Rust router that selects one destination for each HTTP request from ordered conditions on the
request head: method, path, headers, query parameters, HTTP version and trusted HTTPS context. The
first route whose conditions all hold is selected, so a route that fails on a header or a language
lets selection continue. Matching never reads the request body, and an optional Tower adapter
dispatches to Axum handlers or any other Tower service.

**Beta.** The latest release is [`0.1.0-beta.3` on crates.io][crate]. The [documentation
site][site] has guides for each task, and the [API reference] is on docs.rs.

Agents write and maintain Warded. No person reviews the code. See [CONTRIBUTING.md] for
contribution and verification expectations.

## Install

Warded requires Rust 1.99.1 or later. In your Cargo project, add the exact prerelease and the
`http` types used in the example:

```sh
cargo add 'warded@=0.1.0-beta.3' 'http@1.5'
```

This adds the following to `Cargo.toml`:

```toml
[dependencies]
http = "1.5"
warded = "=0.1.0-beta.3"
```

The default features are `tower`, for dispatching to Tower services, and `negotiation`, for the
`Accept` family of headers. To try an unreleased commit instead, pin its full SHA:

```toml
[dependencies]
warded = { git = "https://github.com/viell-dev/slop-warded-router", rev = "<full commit SHA>" }
http = "1.5"
```

[Getting started][getting started] covers features, updates, removal and the repository examples.

## Example

Serve a page in the language the client prefers, falling back to English when the client accepts
none of the offered languages:

```rust
use http::Request;
use warded::{
    Head, Matcher, Selector,
    matchers::{exact_path, get},
    negotiation::{LanguageMatching, Offers},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The languages the page exists in. Lookup lets a request for fr-CA reach fr.
    let languages = Offers::languages(["en", "fr", "de"], LanguageMatching::Lookup)?;
    let welcome = || get().and(exact_path("/welcome"));

    let routes = Selector::new()
        .route(welcome().and(languages.selected("en")?), "English page")
        .route(welcome().and(languages.selected("fr")?), "French page")
        .route(welcome().and(languages.selected("de")?), "German page")
        // Nothing acceptable, or an unreadable header: serve the default.
        .route(welcome(), "English page");

    for (accept_language, page) in [
        ("fr-CA,fr;q=0.9,en;q=0.5", "French page"),
        ("de-AT", "German page"),
        ("es,en;q=0.3", "English page"),
        ("es", "English page"),
    ] {
        let request = Request::builder()
            .uri("/welcome")
            .header("accept-language", accept_language)
            .body(())?;
        let selected = routes.select(&Head::from_request(&request))?;
        assert_eq!(selected.map(|selected| *selected.target), Some(page));
    }
    Ok(())
}
```

Each language route asks whether its language is the client's preferred one among the offers, so
weights such as `q=0.9` decide. `Selector` returns the target without running anything; with the
`tower` feature, `Router` dispatches the same routes to Tower services and adds the captures to
the request. Add `Vary: Accept-Language` to these responses so caches keep the variants apart.

## Documentation

- [Getting started] and [how selection works]: order, misses, errors and combinators.
- [Content negotiation]: languages with a default, JSON or HTML, encodings and their edge cases.
- [Paths and captures], [methods, headers and query parameters] and [transport security].
- [Tower and Axum]: fallbacks, error mapping, readiness and nested routers.
- [Troubleshooting], [scope and limits], [versions] and [upgrading].
- The [API reference] on docs.rs, and the [changelog].

The site describes the latest release; docs.rs keeps the reference for every version.

## Reporting problems

Report defects through [GitHub issues], with the Warded version, Rust version, enabled features, a
synthetic request and route table, and the expected and actual results. Report suspected
vulnerabilities privately as the [security policy] describes.

## License

Licensed under either of [LICENSE-MIT] or [LICENSE-APACHE], at your option.

[crate]: https://crates.io/crates/warded/0.1.0-beta.3
[API reference]: https://docs.rs/warded/0.1.0-beta.3/warded/
[site]: https://viell-dev.github.io/slop-warded-router/
[getting started]: https://viell-dev.github.io/slop-warded-router/getting-started.html
[how selection works]: https://viell-dev.github.io/slop-warded-router/selection.html
[content negotiation]: https://viell-dev.github.io/slop-warded-router/negotiation.html
[paths and captures]: https://viell-dev.github.io/slop-warded-router/paths.html
[methods, headers and query parameters]: https://viell-dev.github.io/slop-warded-router/conditions.html
[transport security]: https://viell-dev.github.io/slop-warded-router/transport-security.html
[Tower and Axum]: https://viell-dev.github.io/slop-warded-router/tower.html
[troubleshooting]: https://viell-dev.github.io/slop-warded-router/troubleshooting.html
[scope and limits]: https://viell-dev.github.io/slop-warded-router/limits.html
[versions]: https://viell-dev.github.io/slop-warded-router/versions.html
[upgrading]: https://viell-dev.github.io/slop-warded-router/upgrading.html
[changelog]: https://viell-dev.github.io/slop-warded-router/changelog.html
[CONTRIBUTING.md]: https://github.com/viell-dev/slop-warded-router/blob/main/CONTRIBUTING.md
[GitHub issues]: https://github.com/viell-dev/slop-warded-router/issues
[security policy]: https://github.com/viell-dev/slop-warded-router/blob/main/SECURITY.md
[LICENSE-MIT]: https://github.com/viell-dev/slop-warded-router/blob/0.1.0-beta.3/LICENSE-MIT
[LICENSE-APACHE]: https://github.com/viell-dev/slop-warded-router/blob/0.1.0-beta.3/LICENSE-APACHE
