//! Ordered routing over complete request-head conditions.
//!
//! Matchers never receive a body. [`Selector`] chooses one target; the optional
//! `Router` Tower adapter executes a selected service without retrying it.
//!
//! ```
//! use http::Request;
//! use warded::{Matcher, Selector, Head, matchers::{exact_path, get}};
//!
//! let selector = Selector::new()
//!     .route(get().and(exact_path("/orders")), "list orders");
//! let request = Request::new(());
//! assert!(selector.select(&Head::from_request(&request)).unwrap().is_none());
//! ```
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
    And, Captures, MatchError, MatchResult, Matcher, Not, Or, match_fn, try_match_fn,
};
pub use matchers::DefinitionError;
pub use selector::{Selected, Selection, Selector, SyncSelector};
#[cfg(feature = "tower")]
pub use service::{DispatchError, RequestDispatchError, RequestRouter, Router};

#[cfg(all(doctest, feature = "tower", feature = "negotiation"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
