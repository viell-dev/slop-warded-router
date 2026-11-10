#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{
        body::Body,
        handler::Handler,
        http::{header, Method, Request, StatusCode},
        response::IntoResponse,
    };
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, render_app_to_stream_with_context};
    use std::convert::Infallible;
    use tower::{make::Shared, service_fn, ServiceExt};
    use warded::{
        matchers::{exact_path, get, head},
        Matcher, TransportSecurity,
    };
    use warded_leptos_probe::app::*;

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Keep this probe explicit: it is not a general Leptos route-list adapter.
    let routes = generate_route_list(App);
    assert_eq!(routes.len(), 1, "expand the fixture before adding routes");
    assert_eq!(routes[0].path(), "/");
    assert!(matches!(
        routes[0].mode(),
        leptos_router::SsrMode::OutOfOrder
    ));
    assert_eq!(
        routes[0].methods().collect::<Vec<_>>(),
        [leptos_router::Method::Get]
    );

    // Supply the same state/method contexts as LeptosRoutes, using its public SSR handler.
    let context_options = leptos_options.clone();
    let render_options = leptos_options.clone();
    let render = render_app_to_stream_with_context(
        move || {
            provide_context(context_options.clone());
            provide_context(leptos_router::Method::Get);
        },
        move || shell(render_options.clone()),
    )
    .with_state(());
    let fallback = service_fn(move |request: Request<Body>| {
        // Construct this opaque, Send-only handler per call; the service captures Sync state.
        leptos_axum::file_and_error_handler(shell)
            .with_state(leptos_options.clone())
            .oneshot(request)
    });
    let app = warded::Router::new()
        .route(get().or(head()).and(exact_path("/")), render)
        // Explicit compatibility policy: Warded itself does not infer HEAD or 405.
        .route(
            exact_path("/"),
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
    let app = service_fn(move |mut request: Request<Body>| {
        let app = app.clone();
        async move {
            let is_head = request.method() == Method::HEAD;
            request.extensions_mut().insert(TransportSecurity::Unsecure);
            let mut response = app.oneshot(request).await.unwrap_or_else(|error| {
                eprintln!("dispatch failed: {error}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            });
            if is_head {
                *response.body_mut() = Body::empty();
            }
            Ok::<_, Infallible>(response)
        }
    });

    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, Shared::new(app)).await.unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
