//! Ordered routing over complete request-head conditions.
//!
//! A route pairs a [`Matcher`] with a target. For each request, routes are tried in
//! declaration order and the first one whose conditions all hold is selected: method,
//! path, headers, query parameters, HTTP version and trusted transport security are equal
//! peers, and a route that fails on any of them lets selection continue. A condition that
//! cannot be evaluated is an error and stops selection instead. Matchers see a [`Head`],
//! which has no access to the request body.
//!
//! [`Selector`] returns the chosen target without running it. With the `tower` feature,
//! `Router` is a Tower service that calls the selected route's service, adds its
//! [`Captures`] and [`Selection`] to the request extensions, and never retries another
//! route after the selected service responds or fails.
//!
//! ```
//! use http::Request;
//! use warded::{Head, Matcher, Selector, matchers::{exact_path, get, path}};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let routes = Selector::new()
//!     .route(get().and(path("/orders/{id}")?), "show order")
//!     .route(get().and(exact_path("/orders")), "list orders");
//!
//! let request = Request::builder().uri("/orders/42").body(())?;
//! let selected = routes.select(&Head::from_request(&request))?.expect("a route matches");
//! assert_eq!(*selected.target, "show order");
//! assert_eq!(selected.selection.captures.get("id"), Some("42"));
//!
//! let request = Request::builder().method("POST").uri("/orders").body(())?;
//! assert!(routes.select(&Head::from_request(&request))?.is_none());
//! # Ok(())
//! # }
//! ```
//!
//! # Guides
//!
//! The [documentation site](https://viell-dev.github.io/slop-warded-router/) explains each
//! task with complete examples. It describes the latest release; this reference covers the
//! version you are reading.
//!
//! - [Getting started](https://viell-dev.github.io/slop-warded-router/getting-started.html)
//!   and [how selection works](https://viell-dev.github.io/slop-warded-router/selection.html)
//! - [Paths and captures](https://viell-dev.github.io/slop-warded-router/paths.html),
//!   [methods, headers and query parameters](https://viell-dev.github.io/slop-warded-router/conditions.html)
//!   and [transport security](https://viell-dev.github.io/slop-warded-router/transport-security.html)
//! - [Content negotiation](https://viell-dev.github.io/slop-warded-router/negotiation.html)
//!   for languages, media types and encodings
//! - [Tower and Axum](https://viell-dev.github.io/slop-warded-router/tower.html) and
//!   [custom matchers](https://viell-dev.github.io/slop-warded-router/custom-matchers.html)
//! - [Troubleshooting](https://viell-dev.github.io/slop-warded-router/troubleshooting.html)
//!
//! # Features
//!
//! Both features are enabled by default:
//!
//! - `tower`: `Router`, `RequestRouter` and their dispatch errors.
//! - `negotiation`: the `negotiation` module for `Accept`, `Accept-Language` and
//!   `Accept-Encoding`.
//!
//! The matcher interface does not expose a request body:
//!
//! ```compile_fail,E0599
//! use warded::match_fn;
//! let matcher = match_fn(|head| head.body().is_empty());
//! ```

mod head;
mod matcher;
mod selector;

pub mod matchers;
#[cfg(feature = "negotiation")]
pub mod negotiation;
#[cfg(feature = "tower")]
mod service;

pub use head::{Head, TransportSecurity};
pub use matcher::{
    And, Captures, Filter, MatchError, MatchResult, Matcher, Not, Or, match_fn, try_match_fn,
};
pub use matchers::DefinitionError;
pub use selector::{Selected, Selection, Selector, SyncSelector};
#[cfg(feature = "tower")]
pub use service::{DispatchError, RequestDispatchError, RequestRouter, Router};

#[cfg(all(doctest, feature = "tower", feature = "negotiation"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
