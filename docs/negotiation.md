# Content negotiation

Clients say which representations they prefer in three request fields:

- `Accept` lists media types, such as `text/html` or `application/json`.
- `Accept-Language` lists languages, such as `fr-CA` or `en`.
- `Accept-Encoding` lists content codings, such as `br` or `gzip`.

Each field is a list, and each member of the list is a _range_: a value the client accepts, such
as `fr-CA`, or a wildcard such as `text/*` or `*` that stands for many values. Each range can carry
a weight from `q=0` to `q=1`, where a higher weight is preferred, the default is 1 and `q=0` means
"not acceptable". A browser might send `Accept-Language: fr-CA,fr;q=0.9,en;q=0.5`: Canadian
French first, any French next, English last.

Negotiation compares that list with what your application offers and picks one offer. Warded turns
the result into a route condition, so routes can serve the chosen representation. This needs the
`negotiation` feature, which is enabled by default. Matching a header with `header_eq` is not
negotiation: it ignores weights and list syntax, so `fr-CA,fr;q=0.9` is not equal to `fr`.

## Two kinds of conditions

Warded has two kinds of negotiation conditions, for different questions:

- **Which of my offers does the client prefer?** Build an offer set with `Offers::languages`,
  `Offers::media_types` or `Offers::encodings`, and give each alternative route
  `offers.selected(offer)`. At most one offer in the set is preferred for a request, so at most
  one of these routes matches, and none does when no offer is acceptable.
- **Is this one offer acceptable at all?** `accepts_language`, `accepts_media_type` and
  `accepts_encoding` match whenever the client would accept that single offer, ignoring any other
  representation you might have. Use them for an endpoint that only produces one representation.

Routes that are alternatives to each other should share one offer set. [Shared offers or
independent conditions](#shared-offers-or-independent-conditions) shows what goes wrong otherwise.

## Serve a page in several languages

This route table serves `/welcome` in English, French or German, and falls back to English when
the client accepts none of them:

```rust
use http::Request;
use std::error::Error;
use warded::{
    Head, Matcher, Selector,
    matchers::{exact_path, get},
    negotiation::{LanguageMatching, Offers},
};

/// Select the welcome page for a request with this Accept-Language field, or none.
fn welcome(
    routes: &Selector<&'static str>,
    accept_language: Option<&str>,
) -> Result<&'static str, Box<dyn Error>> {
    let mut request = Request::builder().uri("/welcome");
    if let Some(value) = accept_language {
        request = request.header("accept-language", value);
    }
    let request = request.body(())?;
    let selected = routes.select(&Head::from_request(&request))?;
    Ok(selected.map_or("no route", |selected| *selected.target))
}

fn main() -> Result<(), Box<dyn Error>> {
    // Every language the page exists in. Without a stated preference, the first is chosen.
    let languages = Offers::languages(["en", "fr", "de"], LanguageMatching::Lookup)?;
    let page = || get().and(exact_path("/welcome"));

    let routes = Selector::new()
        .route(page().and(languages.selected("en")?), "English")
        .route(page().and(languages.selected("fr")?), "French")
        .route(page().and(languages.selected("de")?), "German")
        // Reached when no offered language is acceptable or the field is malformed.
        .route(page(), "English, as the default");

    for (accept_language, expected) in [
        (Some("fr-CA, fr;q=0.9, en;q=0.5"), "French"),
        (Some("de-AT"), "German"),
        (Some("es, en;q=0.3"), "English"),
        (Some("en;q=0.2, fr;q=0.8"), "French"),
        (None, "English"),
        (Some("*"), "English"),
        (Some("en;q=0, *"), "French"),
        (Some("es"), "English, as the default"),
        (Some(""), "English, as the default"),
        (Some("fr;q=2"), "English, as the default"),
    ] {
        assert_eq!(
            welcome(&routes, accept_language)?,
            expected,
            "{accept_language:?}"
        );
    }
    Ok(())
}
```

What happens in each case:

| `Accept-Language`           | Page                  | Why                                                       |
| --------------------------- | --------------------- | --------------------------------------------------------- |
| `fr-CA, fr;q=0.9, en;q=0.5` | French                | `fr-CA` is preferred; lookup falls back from it to `fr`    |
| `de-AT`                     | German                | Lookup falls back from `de-AT` to `de`                    |
| `es, en;q=0.3`              | English               | Spanish is not offered; English is acceptable             |
| `en;q=0.2, fr;q=0.8`        | French                | The higher weight wins                                    |
| absent                      | English               | No preference: the first offer                            |
| `*`                         | English               | Any language: the first offer                             |
| `en;q=0, *`                 | French                | English is excluded; the first remaining offer            |
| `es`                        | English, as the default | Nothing offered is acceptable, so every language route misses |
| empty value                 | English, as the default | The client listed no acceptable language                 |
| `fr;q=2`                    | English, as the default | A weight above 1 is malformed, so the language routes miss |

Three parts of the table do the work:

1. **One offer set.** `languages` lists every language the page exists in. Each language route
   asks whether its language is the preferred one in that set.
2. **An explicit matching mode.** `LanguageMatching::Lookup` lets a request for `de-AT` reach the
   offer `de`. The next section explains the choice.
3. **A final default route.** When nothing is acceptable, or the field cannot be read, every
   language route misses. Without the last route the request would get a 404. That route can
   serve the default language, as here, or a 406 Not Acceptable response.

[A page in several languages with Axum](./tower.md#a-page-in-several-languages-with-axum) serves
this table from an Axum application, with a `Content-Language` header from each handler and a
`Vary: Accept-Language` layer; [caching and Vary](#caching-and-vary) explains the layer.

## Lookup or basic filtering

Language tags have parts: `fr-CA` is French as used in Canada. The two matching modes connect a
client's range and your offer in opposite directions:

- **Lookup** starts from the client's range and shortens it until it reaches an offer: `fr-CA`
  tries `fr-CA`, then `fr`. A request for `fr-CA` can get your `fr`, but a request for `fr` never
  gets your `fr-CA`.
- **Basic filtering** lets a range cover every tag that starts with it: `fr` covers `fr`, `fr-CA`
  and `fr-FR`. A request for `fr` can get your `fr-CA`, but a request for `fr-CA` does not get
  your `fr`.

```rust
use http::Request;
use warded::{
    Head, Matcher,
    negotiation::{LanguageMatching, Offers},
};

/// The offer that the given mode prefers for this Accept-Language field, if any.
fn preferred<'a>(
    offers: &[&'a str],
    mode: LanguageMatching,
    accept_language: &str,
) -> Result<Option<&'a str>, Box<dyn std::error::Error>> {
    let set = Offers::languages(offers.iter().copied(), mode)?;
    let request = Request::builder()
        .header("accept-language", accept_language)
        .body(())?;
    let head = Head::from_request(&request);
    for offer in offers {
        if set.selected(offer)?.evaluate(&head)?.is_some() {
            return Ok(Some(*offer));
        }
    }
    Ok(None)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use LanguageMatching::{BasicFilter, Lookup};

    // General offers: lookup reaches them from specific requests.
    let general = ["en", "fr"];
    assert_eq!(preferred(&general, Lookup, "fr-CA")?, Some("fr"));
    assert_eq!(preferred(&general, BasicFilter, "fr-CA")?, None);

    // Regional offers: basic filtering reaches them from general requests.
    let regional = ["en-US", "fr-CA", "fr-FR"];
    assert_eq!(preferred(&regional, Lookup, "fr")?, None);
    assert_eq!(preferred(&regional, BasicFilter, "fr")?, Some("fr-CA"));
    assert_eq!(preferred(&regional, Lookup, "fr-FR")?, Some("fr-FR"));
    assert_eq!(preferred(&regional, BasicFilter, "fr-BE")?, None);

    // General and regional offers together, with lookup.
    let mixed = ["en", "fr", "fr-CA"];
    assert_eq!(preferred(&mixed, Lookup, "fr-CA")?, Some("fr-CA"));
    assert_eq!(preferred(&mixed, Lookup, "fr-BE")?, Some("fr"));

    // A one-character subtag goes together with the subtag after it.
    let chinese = ["zh", "zh-Hant"];
    assert_eq!(
        preferred(&chinese, Lookup, "zh-Hant-x-priv")?,
        Some("zh-Hant")
    );
    assert_eq!(preferred(&general, Lookup, "fr-x-foo-CA")?, Some("fr"));
    Ok(())
}
```

When lookup shortens a range, it follows RFC 4647: a one-character subtag, such as the `x` that
starts a private-use part, is removed together with the subtag that follows it, so a range never
shortens to a tag that ends in one character. `zh-Hant-x-priv` tries `zh-Hant-x-priv`, then
`zh-Hant` and `zh`; `fr-x-foo-CA` tries `fr-x-foo-CA`, `fr-x-foo` and `fr`.

Which to choose:

- **Use lookup** for most sites. Offer the general languages you have, such as `en` and `fr`, and
  add regional versions such as `fr-CA` only where the content differs. Every regional request
  then reaches the closest offer. Browsers usually list a general language too, as in
  `fr-CA,fr;q=0.9`, but lookup does not depend on it.
- **Use basic filtering** when your offers are regional and a general request should accept any
  of them, for example offers `en-GB` and `en-US` where a request for `en` should get one of them.
  It picks the first matching offer in your list among equal weights.

Warded makes you name the mode because neither is right for every offer set. Lookup and basic
filtering are defined in [RFC 4647](https://www.rfc-editor.org/rfc/rfc4647). Offers must be
well-formed language tags (BCP 47); they are checked when you build the set.

## Weights, wildcards and exclusions

Both modes respect weights, but they break ties and handle `*` differently:

- **Lookup** tries the client's ranges from the highest weight down; ranges of equal weight keep
  the client's order. Each range is shortened all the way before the next range is tried, so
  `fr-CA, en` prefers `fr` over `en`. A positive `*` gives the first offer that is not excluded,
  but only after every named range has been tried, even ranges with a lower weight than `*`.
- **Basic filtering** gives each offer the weight of the longest range that covers it, with `*`
  counting as the shortest. The offer with the highest weight wins; equal weights go to the offer
  listed first in your set.

A `q=0` range excludes offers:

- In basic filtering, an offer whose longest covering range has `q=0` is not acceptable:
  `fr, fr-CA;q=0` accepts `fr-FR` but not `fr-CA`.
- In lookup, a `q=0` range excludes an offer unless the request names that offer more directly.
  `fr;q=0` always excludes `fr`. A more specific exclusion such as `en-GB;q=0` blocks `en` when
  `en` would only be reached by shortening another range, such as `en-US`, or as the `*` default,
  but not when the request names `en` itself.

```rust
use http::Request;
use warded::{
    Head, Matcher,
    negotiation::{LanguageMatching, Offers},
};

/// The offer that the given mode prefers for this Accept-Language field, if any.
fn preferred<'a>(
    offers: &[&'a str],
    mode: LanguageMatching,
    accept_language: &str,
) -> Result<Option<&'a str>, Box<dyn std::error::Error>> {
    let set = Offers::languages(offers.iter().copied(), mode)?;
    let request = Request::builder()
        .header("accept-language", accept_language)
        .body(())?;
    let head = Head::from_request(&request);
    for offer in offers {
        if set.selected(offer)?.evaluate(&head)?.is_some() {
            return Ok(Some(*offer));
        }
    }
    Ok(None)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use LanguageMatching::{BasicFilter, Lookup};

    // Lookup: weight first, then the client's order; each range's fallback comes first.
    assert_eq!(
        preferred(&["en", "fr"], Lookup, "en;q=0.5, fr")?,
        Some("fr")
    );
    assert_eq!(preferred(&["en", "fr"], Lookup, "fr, en")?, Some("fr"));
    assert_eq!(preferred(&["en", "fr"], Lookup, "fr-CA, en")?, Some("fr"));
    // Lookup: the wildcard is the last resort, after lower-weighted named ranges.
    assert_eq!(
        preferred(&["en", "de"], Lookup, "*;q=0.9, de;q=0.1")?,
        Some("de")
    );
    // Basic filtering: equal weights go to the offer listed first.
    assert_eq!(preferred(&["en", "fr"], BasicFilter, "fr, en")?, Some("en"));

    // Exclusions.
    assert_eq!(
        preferred(&["fr-CA", "fr-FR"], BasicFilter, "fr, fr-CA;q=0")?,
        Some("fr-FR")
    );
    assert_eq!(preferred(&["fr", "en"], Lookup, "fr;q=0, *")?, Some("en"));
    assert_eq!(preferred(&["en"], Lookup, "en-US, en-GB;q=0")?, None);
    assert_eq!(preferred(&["en"], Lookup, "en, en-GB;q=0")?, Some("en"));
    Ok(())
}
```

## Absent, empty and malformed headers

A missing field, an empty one and an unreadable one mean different things:

| Field                         | `offers.selected(...)`              | `accepts_*(...)` |
| ----------------------------- | ----------------------------------- | ---------------- |
| Absent                        | The first offer in the set matches  | Matches          |
| Present but empty             | Misses: nothing is acceptable       | Misses           |
| Malformed or over a [limit](#limits-and-cost) | Misses, or errors with `invalid_is_error` | The same |

- **Absent** means the client stated no preference, so any offer will do and a set chooses its
  first. For `Accept-Encoding`, see [encodings](#serve-precompressed-files).
- **Empty** means a list with no members. For `Accept` and `Accept-Language` no offer is
  acceptable, and the route misses even with `invalid_is_error`, because the field is valid.
- **Malformed** input, such as `fr;q=2` or `en_US`, is a miss by default, so a later route, like
  the default route above, can serve the request. Call `invalid_is_error()` on a condition to make
  it an error instead: selection stops with `MatchError::InvalidHeader`, which you can map to a 400
  Bad Request response. Treating malformed input as absent would invent a preference the client
  never stated, so Warded does neither.

Repeated fields are combined into one list in order, so `Accept-Language: fr` followed by
`Accept-Language: en;q=0.5` is read as `fr, en;q=0.5`.

```rust
use http::Request;
use warded::{
    Head, MatchError, Matcher,
    negotiation::{LanguageMatching, accepts_language},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let french = accepts_language("fr", LanguageMatching::Lookup)?;
    let strict = french.clone().invalid_is_error();

    let with_field = |value: Option<&str>| {
        let mut request = Request::builder();
        if let Some(value) = value {
            request = request.header("accept-language", value);
        }
        request.body(())
    };

    let absent = with_field(None)?;
    assert!(french.evaluate(&Head::from_request(&absent))?.is_some());

    let empty = with_field(Some(""))?;
    assert!(french.evaluate(&Head::from_request(&empty))?.is_none());
    assert!(strict.evaluate(&Head::from_request(&empty))?.is_none());

    let malformed = with_field(Some("fr;q=2"))?;
    assert!(french.evaluate(&Head::from_request(&malformed))?.is_none());
    assert!(matches!(
        strict.evaluate(&Head::from_request(&malformed)),
        Err(MatchError::InvalidHeader { .. })
    ));

    // `not` turns the malformed-input miss into a match. Use the strict form when that is wrong.
    assert!(
        french
            .clone()
            .not()
            .evaluate(&Head::from_request(&malformed))?
            .is_some()
    );
    Ok(())
}
```

## Caching and Vary

A response chosen by negotiation depends on a request header, and a shared cache must know that,
or it may give the French page to the next client who asked for English. Add `Vary` with the field
name to every response of a negotiated path, including the default route and any 404 or error
response. Warded does not change responses, so add it in a layer:

```rust
use bytes::Bytes;
use http::{
    Request, Response, StatusCode,
    header::{HeaderValue, VARY},
};
use http_body_util::{BodyExt, Full};
use std::convert::Infallible;
use tower::{ServiceExt, service_fn};
use warded::{
    Matcher, Router,
    matchers::{exact_path, get},
    negotiation::{LanguageMatching, Offers},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let languages = Offers::languages(["en", "fr"], LanguageMatching::Lookup)?;
    let welcome = || get().and(exact_path("/welcome"));
    let text = |body: &'static str| {
        service_fn(move |_: Request<()>| async move {
            Ok::<_, Infallible>(Response::new(Full::new(Bytes::from(body))))
        })
    };

    let app = Router::new()
        .route(welcome().and(languages.selected("en")?), text("Welcome"))
        .route(welcome().and(languages.selected("fr")?), text("Bienvenue"))
        .route(welcome(), text("Welcome"))
        .map_response(|mut response: Response<Full<Bytes>>| {
            // Append, so a Vary set by a destination is kept.
            let vary = HeaderValue::from_static("accept-language");
            response.headers_mut().append(VARY, vary);
            response
        });

    let request = Request::builder()
        .uri("/welcome")
        .header("accept-language", "fr-CH, en;q=0.5")
        .body(())?;
    let response = app.clone().oneshot(request).await?;
    assert_eq!(response.headers()[VARY], "accept-language");
    assert_eq!(
        response.into_body().collect().await?.to_bytes(),
        "Bienvenue"
    );

    // The 404 for an unknown path carries it too.
    let request = Request::builder().uri("/elsewhere").body(())?;
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.headers()[VARY], "accept-language");
    Ok(())
}
```

This layer covers every route for simplicity. To keep other paths cacheable without the header,
wrap only the negotiated path's services. Set `Content-Language`, `Content-Type` or
`Content-Encoding` in each destination, which knows what it produced.

## JSON or HTML from one path

Media types work the same way. Each route asks whether its type is the preferred one:

```rust
use http::Request;
use std::error::Error;
use warded::{
    Head, Matcher, Selector,
    matchers::{exact_path, get},
    negotiation::Offers,
};

fn main() -> Result<(), Box<dyn Error>> {
    let formats = Offers::media_types(["text/html", "application/json"])?;
    let orders = || get().and(exact_path("/orders"));
    let routes = Selector::new()
        .route(orders().and(formats.selected("text/html")?), "HTML page")
        .route(
            orders().and(formats.selected("application/json")?),
            "JSON document",
        )
        .route(orders(), "406 Not Acceptable");

    let browser = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";
    for (accept, expected) in [
        (Some("application/json"), "JSON document"),
        (Some(browser), "HTML page"),
        (Some("application/json, text/html;q=0.5"), "JSON document"),
        (Some("*/*"), "HTML page"),
        (None, "HTML page"),
        (Some("image/png"), "406 Not Acceptable"),
        (Some("text/*, text/html;q=0"), "406 Not Acceptable"),
        // The offer has no charset parameter, so this range does not match it.
        (Some("text/html; charset=utf-8"), "406 Not Acceptable"),
    ] {
        let mut request = Request::builder().uri("/orders");
        if let Some(value) = accept {
            request = request.header("accept", value);
        }
        let request = request.body(())?;
        let selected = routes.select(&Head::from_request(&request))?;
        assert_eq!(
            selected.map(|selected| *selected.target),
            Some(expected),
            "{accept:?}"
        );
    }
    Ok(())
}
```

The rules for media types:

- Each offer takes the weight of the most specific client range that matches it: `text/html`
  beats `text/*`, which beats `*/*`, and a range with more parameters beats one with fewer. That
  is why `text/*, text/html;q=0` excludes `text/html`.
- The offer with the highest weight wins. Equal weights go to the offer listed first in your set,
  so put the representation you prefer to serve first. With `*/*` or no `Accept` field, it is
  chosen.
- A range with parameters matches only offers that carry each of those parameters with the same
  value. Parameter names, and the values of `charset`, ignore ASCII case; other values are compared
  exactly after removing quotes. So `Accept: text/html; charset=utf-8` does not match the offer
  `text/html`, but matches `text/html;charset=UTF-8`. A range without parameters, such as
  `text/html`, matches the offer with or without them.
- Offers are concrete media types and may carry parameters, as in `text/html;level=1`, but not a
  wildcard or a `q` weight.

## Serve precompressed files

`Accept-Encoding` negotiation fits files you have already compressed, such as `app.js.br` and
`app.js.gz`. Warded only chooses; the selected service sends the file and sets `Content-Encoding`.
To compress responses as they are produced, use a compression middleware instead of routes.

```rust
use http::Request;
use std::error::Error;
use warded::{
    Head, Matcher, Selector,
    matchers::{get, path},
    negotiation::Offers,
};

fn main() -> Result<(), Box<dyn Error>> {
    let codings = Offers::encodings(["br", "gzip", "identity"])?;
    let asset = path("/assets/{*file}")?;
    let routes = Selector::new()
        .route(
            get().and(asset.clone()).and(codings.selected("br")?),
            "Brotli file",
        )
        .route(
            get().and(asset.clone()).and(codings.selected("gzip")?),
            "gzip file",
        )
        .route(
            get().and(asset.clone()).and(codings.selected("identity")?),
            "plain file",
        )
        .route(get().and(asset), "406 Not Acceptable");

    for (accept_encoding, expected) in [
        (Some("gzip, br"), "Brotli file"),
        // br has no entry and there is no `*`, so it is not acceptable.
        (Some("gzip"), "gzip file"),
        (Some("x-gzip"), "gzip file"),
        (Some("br;q=0.5, gzip;q=0.8, identity;q=0.1"), "gzip file"),
        // An unlisted identity has no weight, so any acceptable listed coding beats it.
        (Some("br;q=0.5, gzip;q=0.8"), "gzip file"),
        // It is the fallback when no listed coding is acceptable or offered.
        (Some("gzip;q=0"), "plain file"),
        (Some("deflate"), "plain file"),
        (None, "Brotli file"),
        (Some(""), "plain file"),
        (Some("gzip;q=0, *"), "Brotli file"),
        (Some("identity;q=0"), "406 Not Acceptable"),
        (Some("*;q=0"), "406 Not Acceptable"),
    ] {
        let mut request = Request::builder().uri("/assets/app.js");
        if let Some(value) = accept_encoding {
            request = request.header("accept-encoding", value);
        }
        let request = request.body(())?;
        let selected = routes.select(&Head::from_request(&request))?;
        let target = selected.map(|selected| *selected.target);
        assert_eq!(target, Some(expected), "{accept_encoding:?}");
    }
    Ok(())
}
```

Encodings follow the HTTP rules for `Accept-Encoding` in [RFC
9110](https://www.rfc-editor.org/rfc/rfc9110#name-accept-encoding):

- `identity`, meaning no compression, is acceptable unless the field excludes it with
  `identity;q=0`, or with `*;q=0` and no `identity` entry. An `identity` entry gives it a weight
  like any other coding, and without one it takes the weight of `*`. With neither, it has no
  weight: the set selects it only when none of the listed codings you offer is acceptable. That is
  why `br;q=0.5, gzip;q=0.8` selects the gzip file and `deflate` the plain file. When `*` weights
  `identity`, list compressed offers before it so they win ties with it.
  0.1.0-beta.3 gave an unlisted `identity` weight 1 instead, so `br;q=0.5, gzip;q=0.8` selected
  the plain file; see [upgrading](./upgrading.md#from-0-1-0-beta-3).
- An empty `Accept-Encoding` accepts only `identity`.
- Without the field, any coding is acceptable, and the set chooses its first offer.
- A coding takes the weight of its own entry, otherwise the weight of `*`. A coding other than
  `identity` with neither is not acceptable, which is why `gzip` alone selects the gzip file
  although `br` is listed first. Equal weights go to the offer listed first.
- `x-gzip` and `x-compress` are the same codings as `gzip` and `compress`, in the field and in
  your offers, ignoring ASCII case.

## Shared offers or independent conditions

An offer set compares all of its offers and prefers one. Independent `accepts_*` conditions each
check one offer on its own, so several can match the same request, and then declaration order
decides, whatever the weights say:

```rust
use http::Request;
use warded::{
    Head, Selector,
    negotiation::{Offers, accepts_media_type},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request = Request::builder()
        .header("accept", "application/json, text/html;q=0.5")
        .body(())?;
    let head = Head::from_request(&request);

    let formats = Offers::media_types(["text/html", "application/json"])?;
    let shared = Selector::new()
        .route(formats.selected("text/html")?, "html")
        .route(formats.selected("application/json")?, "json");
    assert_eq!(
        *shared
            .select(&head)?
            .expect("one offer is preferred")
            .target,
        "json"
    );

    let independent = Selector::new()
        .route(accepts_media_type("text/html")?, "html")
        .route(accepts_media_type("application/json")?, "json");
    // Both are acceptable, so the first declared route wins.
    assert_eq!(
        *independent
            .select(&head)?
            .expect("both are acceptable")
            .target,
        "html"
    );
    Ok(())
}
```

Share a set only among routes whose other conditions are the same. If one alternative can be
unavailable for a request, because its route has another condition that fails, the set can still
prefer it, and then no alternative matches. The common case is a set reused on several paths that
do not exist in the same languages:

```rust
use http::Request;
use warded::{
    Head, Matcher, Selector,
    matchers::exact_path,
    negotiation::{LanguageMatching, Offers},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request = Request::builder()
        .uri("/blog")
        .header("accept-language", "fr, en;q=0.5")
        .body(())?;
    let head = Head::from_request(&request);

    // The documentation exists in English and French, the blog only in English.
    let docs = Offers::languages(["en", "fr"], LanguageMatching::Lookup)?;
    let blog = Offers::languages(["en"], LanguageMatching::Lookup)?;

    let routes = Selector::new()
        .route(
            exact_path("/docs").and(docs.selected("en")?),
            "docs in English",
        )
        .route(
            exact_path("/docs").and(docs.selected("fr")?),
            "docs in French",
        )
        .route(
            exact_path("/blog").and(blog.selected("en")?),
            "blog in English",
        );
    assert_eq!(
        *routes.select(&head)?.expect("the blog matches").target,
        "blog in English"
    );

    // Reusing the documentation's set on the blog: French is preferred but has no blog route.
    let mistaken = Selector::new().route(
        exact_path("/blog").and(docs.selected("en")?),
        "blog in English",
    );
    assert!(mistaken.select(&head)?.is_none());
    Ok(())
}
```

A route can also require choices from several sets, such as a language and a media type. Each set
is negotiated on its own field.

## Limits and cost

Header fields come from clients, so negotiation is bounded. The defaults, and how to change them:

| Bound                             | Default    | Setting                       |
| --------------------------------- | ---------- | ----------------------------- |
| Combined field bytes              | 64 KiB     | `Limits::max_field_bytes`     |
| Nonempty list members             | 1024       | `Limits::max_list_members`    |
| Parameters per member, with `q`   | 64         | `Limits::max_params` lowers it |
| Empty list elements, such as `,,` | 16         | Fixed                         |

The byte count includes the separators added when repeated fields are combined. A field over a
bound is treated as malformed: it misses, or errors with `invalid_is_error`. Apply smaller limits
to a condition with `parser_limits`:

```rust
use http::Request;
use warded::{
    Head, Matcher,
    negotiation::{LanguageMatching, Limits, Offers},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let limits = Limits::DEFAULT.max_list_members(8).max_field_bytes(1024);
    let languages = Offers::languages(["en", "fr"], LanguageMatching::Lookup)?;
    let french = languages.selected("fr")?.parser_limits(limits);

    let request = Request::builder()
        .header("accept-language", "de, es, it, nl, pt, sv, da, fi, fr")
        .body(())?;
    // Nine members exceed the bound of eight.
    assert!(french.evaluate(&Head::from_request(&request))?.is_none());
    assert!(
        french
            .invalid_is_error()
            .evaluate(&Head::from_request(&request))
            .is_err()
    );
    Ok(())
}
```

Cost depends on how many negotiation conditions a request reaches:

- An offer set negotiates once per request head and limits, and its routes reuse that result.
  Independent `accepts_*` conditions each parse the field again. Cloning a `Head` starts a fresh
  cache.
- Put cheap conditions such as the method and path before negotiation, so routes for other paths
  never parse the field.
- Lower the limits where your clients send short fields. Many independent conditions over fields
  near the default limits are expensive; see [scope and limits](./limits.md#performance).

## What your application decides

Negotiation selects a route; the rest is application policy:

- What to do when nothing is acceptable: a default route, as above, or a 406 response.
- Whether malformed fields are ignored, as by default, or answered with 400 by mapping
  `MatchError::InvalidHeader` from `invalid_is_error`. See [error
  mapping](./tower.md#map-errors-to-responses).
- Response headers: `Content-Type`, `Content-Language`, `Content-Encoding` and `Vary`.
- Compressing or converting the content itself.

The [API reference](https://docs.rs/warded/0.1.0-beta.4/warded/negotiation/index.html) lists the
negotiation types and their options.
