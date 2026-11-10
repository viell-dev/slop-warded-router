use crate::{Head, MatchError, Matcher, Selection, selector::select};
use http::{Request, Response, StatusCode};
use std::{
    fmt,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Service, ServiceExt, util::BoxCloneSyncService};

type Destination<B, R, E> = BoxCloneSyncService<Request<B>, Response<R>, E>;
type Route<B, R, E> = (Arc<dyn Matcher + Send + Sync>, Destination<B, R, E>);

/// A routing or selected-service failure; no variant triggers route reselection.
pub enum DispatchError<B, E> {
    /// A declared predicate could not evaluate the head.
    Match(MatchError),
    /// Explicit error-on-miss mode, retaining the original unread request.
    NoMatch(Request<B>),
    /// The selected destination's readiness or execution failed.
    Service(E),
}
impl<B, E: fmt::Debug> fmt::Debug for DispatchError<B, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Match(e) => f.debug_tuple("Match").field(e).finish(),
            Self::NoMatch(_) => f.write_str("NoMatch(<unread request>)"),
            Self::Service(e) => f.debug_tuple("Service").field(e).finish(),
        }
    }
}
impl<B, E: fmt::Display> fmt::Display for DispatchError<B, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Match(e) => e.fmt(f),
            Self::NoMatch(_) => f.write_str("no route matched"),
            Self::Service(e) => e.fmt(f),
        }
    }
}
impl<B, E: std::error::Error + 'static> std::error::Error for DispatchError<B, E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Match(error) => Some(error),
            Self::Service(error) => Some(error),
            Self::NoMatch(_) => None,
        }
    }
}

/// Ordered request-head routing into cloneable Tower services.
///
/// Readiness waits inside the selected response future, using that service's
/// `oneshot`. Apply an outer concurrency limit for admission control. This router
/// neither buffers bodies nor waits for unrelated services to become ready.
pub struct Router<B, R, E> {
    routes: Vec<Route<B, R, E>>,
    fallback: Option<Destination<B, R, E>>,
    error_on_miss: bool,
}
impl<B, R, E> Clone for Router<B, R, E> {
    fn clone(&self) -> Self {
        Self {
            routes: self.routes.clone(),
            fallback: self.fallback.clone(),
            error_on_miss: self.error_on_miss,
        }
    }
}
impl<B, R, E> Default for Router<B, R, E> {
    fn default() -> Self {
        Self {
            routes: Vec::new(),
            fallback: None,
            error_on_miss: false,
        }
    }
}
impl<B, R, E> Router<B, R, E> {
    /// Start with no routes and the ordinary 404 no-match policy.
    pub fn new() -> Self {
        Self::default()
    }
    /// Append a candidate. All declared conditions participate before dispatch.
    pub fn route<M, S>(mut self, matcher: M, service: S) -> Self
    where
        M: Matcher + Send + Sync + 'static,
        S: Service<Request<B>, Response = Response<R>, Error = E> + Clone + Send + Sync + 'static,
        S::Future: Send + 'static,
    {
        self.routes
            .push((Arc::new(matcher), BoxCloneSyncService::new(service)));
        self
    }
    /// Replace the implicit miss response with the consumer's service.
    ///
    /// A fallback also takes precedence over explicitly enabled error-on-miss.
    pub fn fallback<S>(mut self, service: S) -> Self
    where
        S: Service<Request<B>, Response = Response<R>, Error = E> + Clone + Send + Sync + 'static,
        S::Future: Send + 'static,
    {
        self.fallback = Some(BoxCloneSyncService::new(service));
        self
    }
    /// Return [`DispatchError::NoMatch`] when exhausted without a custom fallback.
    pub fn error_on_miss(mut self) -> Self {
        self.error_on_miss = true;
        self
    }
    /// Inspect applicability independently of service execution and body reads.
    pub fn select(&self, head: &Head<'_>) -> Result<Option<Selection>, MatchError> {
        select(
            self.routes.iter().map(|(m, _)| m.as_ref() as &dyn Matcher),
            head,
        )
    }
}
impl<B, R, E> Service<Request<B>> for Router<B, R, E>
where
    B: Send + 'static,
    R: Default + Send + 'static,
    E: Send + 'static,
{
    type Response = Response<R>;
    type Error = DispatchError<B, E>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        let selection = self.select(&Head::from_request(&request));
        let service = match selection {
            Err(error) => return Box::pin(async move { Err(DispatchError::Match(error)) }),
            Ok(Some(selection)) => {
                let service = self.routes[selection.index].1.clone();
                request.extensions_mut().insert(selection.captures.clone());
                request.extensions_mut().insert(selection);
                Some(service)
            }
            Ok(None) => self.fallback.clone(),
        };
        let error_on_miss = self.error_on_miss;
        Box::pin(async move {
            if let Some(service) = service {
                return service
                    .oneshot(request)
                    .await
                    .map_err(DispatchError::Service);
            }
            if error_on_miss {
                return Err(DispatchError::NoMatch(request));
            }
            let mut response = Response::new(R::default());
            *response.status_mut() = StatusCode::NOT_FOUND;
            Ok(response)
        })
    }
}
