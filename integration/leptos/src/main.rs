#[cfg(feature = "ssr")]
mod server {
    use axum::{
        body::Body,
        handler::Handler,
        http::{header, Method, Request, StatusCode},
        response::{IntoResponse, Response},
    };
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, render_app_to_stream_with_context, LeptosRoutes};
    use std::{convert::Infallible, sync::Arc};
    use tokio::sync::Notify;
    use tower::{make::Shared, service_fn, util::BoxCloneSyncService, ServiceExt};
    use warded::{
        matchers::{exact_path, get, head, header_eq, path, post, query_pair},
        Captures, Matcher, TransportSecurity,
    };
    use warded_leptos_probe::app::*;

    type WebService = BoxCloneSyncService<Request<Body>, Response, Infallible>;

    fn capture_response(render: WebService, kind: &'static str) -> WebService {
        BoxCloneSyncService::new(service_fn(move |request: Request<Body>| {
            let render = render.clone();
            // Read Warded's raw captures in the selected handler, independently of
            // the Leptos router's own parameter extraction during rendering.
            let raw = request
                .extensions()
                .get::<Captures>()
                .and_then(|captures| captures.get("value"))
                .expect("selected user route supplies a capture")
                .to_owned();
            async move {
                let mut response = render.oneshot(request).await?;
                response
                    .headers_mut()
                    .insert("x-route-kind", kind.parse().unwrap());
                response
                    .headers_mut()
                    .insert("x-capture", raw.parse().unwrap());
                Ok(response)
            }
        }))
    }

    pub async fn run() {
        let options = get_configuration(None).unwrap().leptos_options;
        let addr = options.site_addr;
        let gate = Arc::new(Notify::new());
        let routes = generate_route_list(App);
        let mut paths = routes.iter().map(|route| route.path()).collect::<Vec<_>>();
        paths.sort();
        assert_eq!(paths, ["/", "/form", "/stream", "/users/{value}"]);
        for route in &routes {
            assert!(matches!(route.mode(), leptos_router::SsrMode::OutOfOrder));
            assert_eq!(
                route.methods().collect::<Vec<_>>(),
                [leptos_router::Method::Get]
            );
        }

        let context_gate = gate.clone();
        let context_options = options.clone();
        let context = move || {
            provide_context(context_gate.clone());
            provide_context(context_options.clone());
            provide_context(leptos_router::Method::Get);
        };
        let render_options = options.clone();
        let render = BoxCloneSyncService::new(
            render_app_to_stream_with_context(context.clone(), move || {
                shell(render_options.clone())
            })
            .with_state(()),
        );
        let fallback_options = options.clone();
        let fallback = service_fn(move |request: Request<Body>| {
            // This public opaque handler is Send-only: create it per request,
            // keeping the service's captured state Sync.
            leptos_axum::file_and_error_handler(shell)
                .with_state(fallback_options.clone())
                .oneshot(request)
        });
        let release_gate = gate.clone();
        let release = service_fn(move |_: Request<Body>| {
            release_gate.notify_one();
            async { Ok::<_, Infallible>(StatusCode::NO_CONTENT.into_response()) }
        });
        let failure = service_fn(|_: Request<Body>| async {
            Ok::<_, Infallible>(
                (StatusCode::INTERNAL_SERVER_ERROR, "selected failure").into_response(),
            )
        });
        let mode = std::env::var("WARDED_FIXTURE_ROUTER").unwrap_or_else(|_| "warded".into());
        let app: WebService = match mode.as_str() {
            "axum" => {
                let baseline = axum::Router::new()
                    .leptos_routes_with_context(&options, routes, context, {
                        let options = options.clone();
                        move || shell(options.clone())
                    })
                    .route_service("/__test/release-stream", release)
                    .route_service("/__test/failure", failure)
                    .fallback(leptos_axum::file_and_error_handler(shell))
                    .with_state(options);
                BoxCloneSyncService::new(baseline)
            }
            "warded" => {
                let reject_id = match std::env::var("WARDED_FIXTURE_MUTATION").as_deref() {
                    Ok("reject-id") => true,
                    Err(std::env::VarError::NotPresent) => false,
                    other => panic!("unknown fixture mutation: {other:?}"),
                };
                let functions = service_fn(move |request: Request<Body>| {
                    let context = context.clone();
                    async move {
                        Ok::<_, Infallible>(
                            leptos_axum::handle_server_fns_with_context(context, request)
                                .await
                                .into_response(),
                        )
                    }
                });
                let app = warded::Router::new()
                    .route(post().and(exact_path("/__test/release-stream")), release)
                    .route(get().and(exact_path("/__test/failure")), failure)
                    // A response from the selected service must never select this.
                    .route(
                        get().and(exact_path("/__test/failure")),
                        service_fn(|_: Request<Body>| async {
                            Ok::<_, Infallible>("incorrect reselection".into_response())
                        }),
                    )
                    .route(post().and(path("/api/{*function}").unwrap()), functions)
                    .route(
                        post().and(path("/api/{*function}").unwrap()),
                        service_fn(|_: Request<Body>| async {
                            Ok::<_, Infallible>(
                                "incorrect server-function reselection".into_response(),
                            )
                        }),
                    )
                    .route(
                        get()
                            .and(exact_path("/variants"))
                            .and(header_eq("x-mode", "preview").unwrap())
                            .and(query_pair("kind", "summary")),
                        service_fn(|request: Request<Body>| async move {
                            Ok::<_, Infallible>(
                                (
                                    [(header::VARY, "x-mode")],
                                    format!("preview:{}", request.uri()),
                                )
                                    .into_response(),
                            )
                        }),
                    )
                    .route(
                        get().and(exact_path("/variants")),
                        service_fn(|_: Request<Body>| async {
                            Ok::<_, Infallible>(
                                ([(header::VARY, "x-mode")], "default").into_response(),
                            )
                        }),
                    )
                    .route(
                        get().or(head()).and(path("/users/{value}").unwrap().filter(
                            move |captures| {
                                !reject_id
                                    && captures
                                        .get("value")
                                        .is_some_and(|value| value.parse::<UserId>().is_ok())
                            },
                        )),
                        capture_response(render.clone(), "id"),
                    )
                    .route(
                        get().or(head()).and(
                            path("/users/{value}")
                                .unwrap()
                                .filter(|captures| captures.get("value").is_some_and(is_username)),
                        ),
                        capture_response(render.clone(), "username"),
                    )
                    // The framework fallback must not re-admit a rejected value.
                    .route(
                        get().or(head()).and(path("/users/{value}").unwrap()),
                        service_fn(|_: Request<Body>| async {
                            Ok::<_, Infallible>(
                                (StatusCode::NOT_FOUND, "No matching user value").into_response(),
                            )
                        }),
                    )
                    .route(
                        get().or(head()).and(
                            exact_path("/")
                                .or(exact_path("/form"))
                                .or(exact_path("/stream")),
                        ),
                        render,
                    )
                    // This fixture explicitly supplies the baseline's method policy.
                    .route(
                        exact_path("/")
                            .or(exact_path("/form"))
                            .or(exact_path("/stream"))
                            .or(path("/users/{value}").unwrap()),
                        service_fn(|_: Request<Body>| async {
                            Ok::<_, Infallible>(
                                (
                                    StatusCode::METHOD_NOT_ALLOWED,
                                    [(header::ALLOW, "GET,HEAD")],
                                )
                                    .into_response(),
                            )
                        }),
                    )
                    .fallback(fallback);
                BoxCloneSyncService::new(service_fn(move |mut request: Request<Body>| {
                    let app = app.clone();
                    request.extensions_mut().insert(TransportSecurity::Unsecure);
                    async move {
                        Ok::<_, Infallible>(app.oneshot(request).await.unwrap_or_else(|error| {
                            eprintln!("dispatch failed: {error}");
                            StatusCode::INTERNAL_SERVER_ERROR.into_response()
                        }))
                    }
                }))
            }
            other => panic!("unknown router mode: {other}"),
        };
        let app = service_fn(move |request: Request<Body>| {
            let app = app.clone();
            async move {
                let is_head = request.method() == Method::HEAD;
                let mut response = app.oneshot(request).await?;
                if is_head {
                    *response.body_mut() = Body::empty();
                }
                Ok::<_, Infallible>(response)
            }
        });
        println!("fixture router={mode} listening on http://{addr}");
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, Shared::new(app)).await.unwrap();
    }
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    server::run().await;
}

#[cfg(not(feature = "ssr"))]
fn main() {}
