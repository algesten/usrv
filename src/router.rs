use std::marker::PhantomData;

use crate::handler::Handler;
use crate::http::{Method, Request, Response};
use crate::matcher::request_matcher;
use crate::{Body, SendBody, Service};

/// A builder for registering routes and creating a callable router.
///
/// Routes are registered per-method using convenience functions like
/// [`Router::get`] and [`Router::post`].
///
/// # Example
///
/// ```no_run
/// use usrv::{http, Router};
///
/// fn hello() -> &'static str { "hello" }
///
/// let router = Router::new()
///     .get("/hello", hello)
///     .build();
///
/// let req = http::Request::builder().uri("/hello").body(usrv::Body).unwrap();
/// let _resp = router.call((), req);
/// ```
pub struct Router<S = ()> {
    _state: PhantomData<S>,
}

impl Router {
    /// Create a new empty router without application state.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use usrv::Router;
    ///
    /// fn hello() -> &'static str { "hello" }
    ///
    /// let _router = Router::new().get("/hello", hello).build();
    /// ```
    pub fn new() -> Self {
        Self::with_state::<()>()
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
    /// use usrv::Router;
    ///
    /// struct App;
    ///
    /// fn handler(_app: &mut App) { /* ... */ }
    ///
    /// let _router = Router::with_state::<&mut App>()
    ///     .get("/", handler)
    ///     .build();
    /// ```
    pub fn with_state<S>() -> Router<S> {
        Router {
            _state: PhantomData,
        }
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[doc(hidden)]
pub trait Callable<S>: Clone {
    fn call(&self, state: S, request: Request<Body>) -> CallResult<S>;
}

#[doc(hidden)]
pub enum CallResult<S> {
    Handled(Response<SendBody>),
    Unhandled(S, Request<Body>),
}

impl<S> Callable<S> for Router<S> {
    fn call(&self, state: S, request: Request<Body>) -> CallResult<S> {
        CallResult::Unhandled(state, request)
    }
}

/// Provides method-based route registration and router construction.
impl<S> Router<S> {
    /// Finalize the route definitions and create a callable router.
    pub fn build(self) -> Service<S, Self> where Self: Callable<S> {
        Service::new(self)
    }

    /// Register a handler for a specific HTTP method and path.
    pub fn handle<'a, T, H: Handler<T, S>>(
        self,
        method: Method,
        path: &'a str,
        handler: H,
    ) -> RouterChain<'a, T, S, H, Self> {
        RouterChain {
            _htype: PhantomData,
            _state: PhantomData,
            parent: self,
            method,
            path,
            handler,
        }
    }

    /// Register a GET handler.
    pub fn get<'a, T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::GET, path, handler)
    }

    /// Register a POST handler.
    pub fn post<'a, T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::POST, path, handler)
    }

    /// Register a PUT handler.
    pub fn put<'a, T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::PUT, path, handler)
    }

    /// Register a DELETE handler.
    pub fn delete<'a, T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::DELETE, path, handler)
    }

    /// Register a HEAD handler.
    pub fn head<'a, T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::HEAD, path, handler)
    }

    /// Register an OPTIONS handler.
    pub fn options<'a, T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::OPTIONS, path, handler)
    }

    /// Register a CONNECT handler.
    pub fn connect<'a, T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::CONNECT, path, handler)
    }

    /// Register a PATCH handler.
    pub fn patch<'a, T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::PATCH, path, handler)
    }

    /// Register a TRACE handler.
    pub fn trace<'a, T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::TRACE, path, handler)
    }
}

/// A chained route definition.
///
/// Returned by the method-specific registration functions and supports chaining
/// additional registrations before calling `build`.
pub struct RouterChain<'a, T, S, H, P> {
    _htype: PhantomData<T>,
    _state: PhantomData<S>,
    parent: P,
    method: Method,
    path: &'a str,
    handler: H,
}

impl<'a, T, S, H: Handler<T, S>, P: Callable<S>> Callable<S> for RouterChain<'a, T, S, H, P> {
    fn call(&self, state: S, request: Request<Body>) -> CallResult<S> {
        // First call parent since that reflects the order the handlers are declared.
        match self.parent.call(state, request) {
            // Parent handled request, pass response on
            CallResult::Handled(r) => CallResult::Handled(r),

            // Parent did not handle request
            CallResult::Unhandled(state, request) => {
                // Try to match to our path
                if request_matcher(&request, &self.method, self.path) {
                    // Run our handler
                    let result = self.handler.clone().call(state, request);

                    // Result is now handled
                    CallResult::Handled(result)
                } else {
                    // Path doesn't match, we are not to run the handler
                    CallResult::Unhandled(state, request)
                }
            }
        }
    }
}

impl<'a, T1, S, H1: Handler<T1, S>, P1: Callable<S>> RouterChain<'a, T1, S, H1, P1> {
    /// Finalize the route definitions and create a callable router.
    pub fn build(self) -> Service<S, Self> where Self: Callable<S> {
        Service::new(self)
    }

    /// Register a handler for a specific HTTP method and path.
    pub fn handle<T, H: Handler<T, S>>(
        self,
        method: Method,
        path: &'a str,
        handler: H,
    ) -> RouterChain<'a, T, S, H, Self> {
        RouterChain {
            _htype: PhantomData,
            _state: PhantomData,
            parent: self,
            method,
            path,
            handler,
        }
    }

    /// Register a GET handler.
    pub fn get<T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::GET, path, handler)
    }

    /// Register a POST handler.
    pub fn post<T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::POST, path, handler)
    }

    /// Register a PUT handler.
    pub fn put<T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::PUT, path, handler)
    }

    /// Register a DELETE handler.
    pub fn delete<T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::DELETE, path, handler)
    }

    /// Register a HEAD handler.
    pub fn head<T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::HEAD, path, handler)
    }

    /// Register an OPTIONS handler.
    pub fn options<T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::OPTIONS, path, handler)
    }

    /// Register a CONNECT handler.
    pub fn connect<T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::CONNECT, path, handler)
    }

    /// Register a PATCH handler.
    pub fn patch<T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::PATCH, path, handler)
    }

    /// Register a TRACE handler.
    pub fn trace<T, H: Handler<T, S>>(self, path: &'a str, handler: H) -> RouterChain<'a, T, S, H, Self> {
        Self::handle(self, Method::TRACE, path, handler)
    }
}

impl<S> Clone for Router<S> {
    fn clone(&self) -> Self {
        Self {
            _state: PhantomData,
        }
    }
}

impl<'a, T, S, H: Clone, P: Clone> Clone for RouterChain<'a, T, S, H, P> {
    fn clone(&self) -> Self {
        Self {
            _htype: PhantomData,
            _state: PhantomData,
            parent: self.parent.clone(),
            method: self.method.clone(),
            path: self.path,
            handler: self.handler.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn make_route() {
        fn is_send<T: Send>(_t: T) {}

        struct AppState;

        fn root() {}
        fn root_ret() -> &'static str {
            "hi"
        }

        fn req(_r: Request<Body>) {}
        fn req_ret(_r: Request<Body>) -> &'static str {
            "hi"
        }

        fn app(_s: &mut AppState) {}
        fn app_ret(_s: &mut AppState) -> &str {
            "hi"
        }

        fn app_req(_s: &mut AppState, _r: Request<Body>) {}
        fn app_req_ret(_s: &mut AppState, _r: Request<Body>) -> &str {
            "hi"
        }

        let router = Router::with_state::<&mut AppState>()
            .get("/", root)
            .get("/x", root_ret)
            .get("/req", req)
            .get("/req_x", req_ret)
            .get("/app", app)
            .get("/app_x", app_ret)
            .get("/app_req", app_req)
            .get("/app_req_x", app_req_ret)
            .get("free", |_r: Request<Body>| {})
            .build();

        let mut state = AppState;

        let _respone= router.call(&mut state, Request::builder().uri("/").body(Body).unwrap());

        is_send(router);
    }
}
