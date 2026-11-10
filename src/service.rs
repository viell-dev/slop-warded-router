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
#[non_exhaustive]
pub enum DispatchError<E> {
    /// A declared predicate could not evaluate the head.
    Match(MatchError),
    /// The selected destination's readiness or execution failed.
    Service(E),
}
impl<E: fmt::Debug> fmt::Debug for DispatchError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Match(e) => f.debug_tuple("Match").field(e).finish(),
            Self::Service(e) => f.debug_tuple("Service").field(e).finish(),
        }
    }
}
impl<E: fmt::Display> fmt::Display for DispatchError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Match(e) => e.fmt(f),
            Self::Service(e) => e.fmt(f),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for DispatchError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Match(error) => Some(error),
            Self::Service(error) => Some(error),
        }
    }
}

/// Ordered request-head routing into cloneable Tower services.
///
/// Readiness waits inside the selected response future, using that service's
/// `oneshot`. Apply an outer concurrency limit for admission control. This router
/// neither buffers bodies nor waits for unrelated services to become ready.
pub struct Router<B, R, E> {
    table: Arc<RouteTable<B, R, E>>,
}

struct RouteTable<B, R, E> {
    routes: Vec<Route<B, R, E>>,
    fallback: Option<Destination<B, R, E>>,
}
impl<B, R, E> Clone for RouteTable<B, R, E> {
    fn clone(&self) -> Self {
        Self {
            routes: self.routes.clone(),
            fallback: self.fallback.clone(),
        }
    }
}
impl<B, R, E> Clone for Router<B, R, E> {
    fn clone(&self) -> Self {
        Self {
            table: self.table.clone(),
        }
    }
}
impl<B, R, E> Default for Router<B, R, E> {
    fn default() -> Self {
        Self {
            table: Arc::new(RouteTable {
                routes: Vec::new(),
                fallback: None,
            }),
        }
    }
}
impl<B, R, E> Router<B, R, E> {
    /// Start with no routes and the ordinary 404 no-match policy.
    pub fn new() -> Self {
        Self::default()
    }
    /// Append a candidate. All declared conditions participate before dispatch.
    ///
    /// Editing a cloned router copies its table; cloning for dispatch does not.
    #[must_use]
    pub fn route<M, S>(mut self, matcher: M, service: S) -> Self
    where
        M: Matcher + Send + Sync + 'static,
        S: Service<Request<B>, Response = Response<R>, Error = E> + Clone + Send + Sync + 'static,
        S::Future: Send + 'static,
    {
        Arc::make_mut(&mut self.table)
            .routes
            .push((Arc::new(matcher), BoxCloneSyncService::new(service)));
        self
    }
    /// Replace the implicit miss response with the consumer's service.
    ///
    /// A fallback also takes precedence over explicitly enabled error-on-miss.
    #[must_use]
    pub fn fallback<S>(mut self, service: S) -> Self
    where
        S: Service<Request<B>, Response = Response<R>, Error = E> + Clone + Send + Sync + 'static,
        S::Future: Send + 'static,
    {
        Arc::make_mut(&mut self.table).fallback = Some(BoxCloneSyncService::new(service));
        self
    }
    /// Return the unread request on exhaustion using a separate service/error type.
    ///
    /// This adapter does not require a default response body. A custom fallback
    /// still takes precedence over returning the request.
    #[must_use]
    pub fn error_on_miss(self) -> RequestRouter<B, R, E> {
        RequestRouter(self)
    }
    /// Inspect applicability independently of service execution and body reads.
    pub fn select(&self, head: &Head<'_>) -> Result<Option<Selection>, MatchError> {
        select(
            self.table
                .routes
                .iter()
                .map(|(m, _)| m.as_ref() as &dyn Matcher),
            head,
        )
    }

    fn destination(
        &self,
        request: &mut Request<B>,
    ) -> Result<Option<Destination<B, R, E>>, MatchError> {
        match self.select(&Head::from_request(request))? {
            Some(selection) => {
                let service = self.table.routes[selection.index].1.clone();
                request.extensions_mut().insert(selection.captures.clone());
                request.extensions_mut().insert(selection);
                Ok(Some(service))
            }
            None => Ok(self.table.fallback.clone()),
        }
    }
}
impl<B, R, E> Service<Request<B>> for Router<B, R, E>
where
    B: Send + 'static,
    R: Default + Send + 'static,
    E: Send + 'static,
{
    type Response = Response<R>;
    type Error = DispatchError<E>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        let service = match self.destination(&mut request) {
            Err(error) => return Box::pin(async move { Err(DispatchError::Match(error)) }),
            Ok(service) => service,
        };
        Box::pin(async move {
            if let Some(service) = service {
                return service
                    .oneshot(request)
                    .await
                    .map_err(DispatchError::Service);
            }
            let mut response = Response::new(R::default());
            *response.status_mut() = StatusCode::NOT_FOUND;
            Ok(response)
        })
    }
}

/// A request-preserving dispatch error used only by [`RequestRouter`].
#[non_exhaustive]
pub enum RequestDispatchError<B, E> {
    /// No route or custom fallback was applicable; the body remains unread.
    NoMatch(Request<B>),
    /// A matcher or selected service failed. No alternative is tried.
    Dispatch(DispatchError<E>),
}

impl<B, E: fmt::Debug> fmt::Debug for RequestDispatchError<B, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMatch(_) => f.write_str("NoMatch(<unread request>)"),
            Self::Dispatch(error) => error.fmt(f),
        }
    }
}
impl<B, E: fmt::Display> fmt::Display for RequestDispatchError<B, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMatch(_) => f.write_str("no route matched"),
            Self::Dispatch(error) => error.fmt(f),
        }
    }
}
impl<B, E: std::error::Error + 'static> std::error::Error for RequestDispatchError<B, E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoMatch(_) => None,
            Self::Dispatch(error) => Some(error),
        }
    }
}

/// An explicit no-match adapter, created by [`Router::error_on_miss`].
///
/// The normal [`Router`] error contains no body, so it can be boxed even when
/// the request body is not `Sync`. Only this opt-in adapter retains requests.
pub struct RequestRouter<B, R, E>(Router<B, R, E>);

impl<B, R, E> Clone for RequestRouter<B, R, E> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<B, R, E> RequestRouter<B, R, E> {
    /// Append a candidate before returning unmatched requests.
    #[must_use]
    pub fn route<M, S>(self, matcher: M, service: S) -> Self
    where
        M: Matcher + Send + Sync + 'static,
        S: Service<Request<B>, Response = Response<R>, Error = E> + Clone + Send + Sync + 'static,
        S::Future: Send + 'static,
    {
        Self(self.0.route(matcher, service))
    }

    /// Use a custom fallback before returning an unmatched request.
    #[must_use]
    pub fn fallback<S>(self, service: S) -> Self
    where
        S: Service<Request<B>, Response = Response<R>, Error = E> + Clone + Send + Sync + 'static,
        S::Future: Send + 'static,
    {
        Self(self.0.fallback(service))
    }

    /// Inspect applicability without executing a service.
    pub fn select(&self, head: &Head<'_>) -> Result<Option<Selection>, MatchError> {
        self.0.select(head)
    }
}

impl<B: Send + 'static, R: Send + 'static, E: Send + 'static> Service<Request<B>>
    for RequestRouter<B, R, E>
{
    type Response = Response<R>;
    type Error = RequestDispatchError<B, E>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        let service = self.0.destination(&mut request);
        Box::pin(async move {
            match service {
                Err(error) => Err(RequestDispatchError::Dispatch(DispatchError::Match(error))),
                Ok(None) => Err(RequestDispatchError::NoMatch(request)),
                Ok(Some(service)) => service
                    .oneshot(request)
                    .await
                    .map_err(|error| RequestDispatchError::Dispatch(DispatchError::Service(error))),
            }
        })
    }
}
