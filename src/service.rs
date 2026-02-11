use crate::http::{Request, Response};
use crate::router::{BuiltRouter, CallResult, Callable, Router};
use crate::{Body, IntoResponse, NotFound, SendBody};

/// A callable service produced from a router.
///
/// Build using `Router::build`, then call with a state and request to
/// produce a response.
pub struct Service<S> {
    router: BuiltRouter<S>,
}

impl Service<()> {
    /// Create a new empty router without application state.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use usrv::Service;
    ///
    /// fn hello() -> &'static str { "hello" }
    ///
    /// let _router = Service::router().get("/hello", hello).build();
    /// ```
    pub fn router() -> Router<()> {
        Router::new()
    }

    /// Create a new empty router parameterized by application state `S`.
    ///
    /// The state is passed by value to handlers that declare it as the first
    /// parameter. A common pattern is to use a mutable reference to a shared
    /// state type, e.g. `&mut AppState`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use usrv::Service;
    ///
    /// struct App;
    ///
    /// fn handler(_app: &mut App) { /* ... */ }
    ///
    /// let _router = Service::with_state::<&mut App>()
    ///     .get("/", handler)
    ///     .build();
    /// ```
    pub fn with_state<S>() -> Router<S> {
        Router::with_state()
    }
}

impl<S> Service<S> {
    pub(crate) fn new(router: BuiltRouter<S>) -> Self {
        Service { router }
    }

    /// Call the service with the given state and request.
    pub fn call(&self, state: S, request: Request<Body>) -> Response<SendBody> {
        match self.router.call(state, request) {
            CallResult::Handled(v) => v,
            CallResult::Unhandled(_, _) => NotFound.into_response(),
        }
    }
}

impl<S> Clone for Service<S> {
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
        }
    }
}
