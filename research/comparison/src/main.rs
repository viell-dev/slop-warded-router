use actix_web::{App, HttpResponse, guard, test, web};
use rama_core::Service as _;
use rama_http::matcher::HttpMatcher;
use salvo::{Router, Service, test::TestClient};
use warp::Filter;

#[salvo::handler]
async fn ok(res: &mut salvo::Response) {
    res.status_code(salvo::http::StatusCode::OK);
}

#[tokio::main]
async fn main() {
    let actix_native = test::init_service(
        App::new().service(
            web::resource("/orders").route(
                web::route()
                    .guard(guard::Header("accept-language", "fr"))
                    .to(HttpResponse::Ok),
            ),
        ),
    )
    .await;
    let actix_configured = test::init_service(
        App::new().service(
            web::resource("/orders")
                .guard(guard::Header("accept-language", "fr"))
                .to(HttpResponse::Ok),
        ),
    )
    .await;
    let warp = warp::path("orders")
        .and(warp::path::end())
        .and(warp::header::exact("accept-language", "fr"))
        .map(warp::reply);
    let salvo = Service::new(
        Router::new().push(
            Router::with_path("orders")
                .filter_fn(|req, _| {
                    req.headers()
                        .get("accept-language")
                        .is_some_and(|v| v == "fr")
                })
                .goal(ok),
        ),
    );
    let rama = rama_http::service::web::match_service! {
        HttpMatcher::path("/orders").and_header(
            rama_http::header::ACCEPT_LANGUAGE,
            rama_http::HeaderValue::from_static("fr"),
        ) => rama_http::StatusCode::OK,
        _ => rama_http::StatusCode::NOT_FOUND,
    };

    println!("path,language,actix_route_guard,actix_resource_guard,warp,salvo,rama");
    for (path, lang, expected) in [
        ("/orders", Some("fr"), [200, 200, 200, 200, 200]),
        ("/orders", Some("en"), [405, 404, 400, 404, 404]),
        ("/orders", None, [405, 404, 400, 404, 404]),
        ("/orders", Some("fr-FR"), [405, 404, 400, 404, 404]),
        ("/other", Some("fr"), [404, 404, 404, 404, 404]),
    ] {
        let actix_req = || {
            let mut request = test::TestRequest::with_uri(path);
            if let Some(lang) = lang {
                request = request.insert_header(("accept-language", lang));
            }
            request.to_request()
        };
        let a = test::call_service(&actix_native, actix_req())
            .await
            .status()
            .as_u16();
        let b = test::call_service(&actix_configured, actix_req())
            .await
            .status()
            .as_u16();
        let mut request = warp::test::request().path(path);
        if let Some(lang) = lang {
            request = request.header("accept-language", lang);
        }
        let w = request.reply(&warp).await.status().as_u16();
        let mut request = TestClient::get(format!("http://example.test{path}"));
        if let Some(lang) = lang {
            request = request.add_header("accept-language", lang, true);
        }
        let s = request.send(&salvo).await.status_code.unwrap().as_u16();
        let mut request = rama_http::Request::builder().uri(path);
        if let Some(lang) = lang {
            request = request.header("accept-language", lang);
        }
        let r = rama
            .serve(request.body(rama_http::Body::empty()).unwrap())
            .await
            .unwrap()
            .status()
            .as_u16();
        let actual = [a, b, w, s, r];
        assert_eq!(actual, expected, "{path} {lang:?}");
        println!("{path},{},{a},{b},{w},{s},{r}", lang.unwrap_or("absent"));
    }

    let alternatives = test::init_service(
        App::new()
            .service(
                web::resource("/orders")
                    .guard(guard::Header("accept-language", "fr"))
                    .to(HttpResponse::Ok),
            )
            .service(
                web::resource("/orders")
                    .guard(guard::Header("accept-language", "en"))
                    .to(HttpResponse::Accepted),
            ),
    )
    .await;
    let response = test::call_service(
        &alternatives,
        test::TestRequest::with_uri("/orders")
            .insert_header(("accept-language", "en"))
            .to_request(),
    )
    .await;
    assert_eq!(response.status().as_u16(), 202);
    println!("actix resource-guard same-path alternative: 202");

    let overlap = test::init_service(
        App::new()
            .service(
                web::resource("/orders/new")
                    .guard(guard::Header("accept-language", "fr"))
                    .to(HttpResponse::Ok),
            )
            .service(web::resource("/orders/{id}").to(HttpResponse::Accepted)),
    )
    .await;
    let response = test::call_service(
        &overlap,
        test::TestRequest::with_uri("/orders/new")
            .insert_header(("accept-language", "en"))
            .to_request(),
    )
    .await;
    assert_eq!(response.status().as_u16(), 202);
    println!("actix resource-guard static/parameter fallthrough: 202");

    negotiation_probes();
}

fn negotiation_probes() {
    use parse_net::negotiate::{LanguageMatching, MalformedPolicy, Negotiator, Outcome};
    let n = Negotiator::new().on_malformed(MalformedPolicy::Refuse);
    for (header, offers, expected) in [
        (
            "application/json;q=1,text/html;q=0.5",
            vec!["text/html", "application/json"],
            Some(1),
        ),
        (
            "text/*;q=1,text/html;q=0",
            vec!["text/html", "text/plain"],
            Some(1),
        ),
        (
            "text/html;level=1",
            vec!["text/html;level=2", "text/html;level=1"],
            Some(1),
        ),
        ("application/json;q=0", vec!["application/json"], None),
    ] {
        assert_eq!(
            n.media(Some(header.as_bytes()), &offers).index(),
            expected,
            "{header}"
        );
    }
    assert!(matches!(
        n.media(Some(b"text/html;q=2"), &["text/html"]),
        Outcome::Refused(_)
    ));
    assert_eq!(n.media(None, &["text/html"]).index(), Some(0));
    assert_eq!(n.encoding(None, &["gzip"]).index(), Some(0));
    assert_eq!(n.encoding(Some(b""), &["gzip"]).index(), None);
    assert_eq!(
        n.encoding(Some(b"identity;q=0,*;q=0"), &["identity", "gzip"])
            .index(),
        None
    );
    assert_eq!(
        n.language_matching(LanguageMatching::BasicFilter)
            .language(Some(b"fr"), &["fr-CA"])
            .index(),
        Some(0)
    );
    assert_eq!(n.language(Some(b"fr-CA"), &["fr"]).index(), Some(0));
    assert_eq!(
        n.media_lines(
            [
                b"text/html;q=0.5".as_slice(),
                b"application/json".as_slice()
            ],
            &["text/html", "application/json"]
        )
        .index(),
        Some(1)
    );
    assert!("text/html;q=2".parse::<headers_accept::Accept>().is_ok());
    assert!(negotiator::preferred_encodings(None, Some(&["gzip"]), None).is_empty());
    println!(
        "parse-net: media preference, q=0 specificity, media parameters, malformed q, repeated fields, language schemes, absent/empty encoding and identity: passed"
    );
    println!(
        "candidate differences: headers-accept accepts invalid q; negotiator absent encoding offer gzip is not selected"
    );
}
