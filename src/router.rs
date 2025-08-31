//! Method-based route registration and router construction.
//!
//! You typically create a router using [`Service::router`]
//! or [`Service::with_state`].
use std::collections::HashMap;
use std::sync::Arc;

use crate::handler::Handler;
use crate::http::{Method, Request, Response};
use crate::{Body, SendBody, Service};

/// Type-erased handler function stored by the router.
type HandlerFn<S> = Arc<dyn Fn(S, Request<Body>) -> Response<SendBody> + Send + Sync + 'static>;

/// Builder for registering routes that compiles into a callable router.
///
/// Use the method helpers (for example `get`, `post`) to register routes, then
/// call `build` to produce a `Service` that can handle requests.
pub struct Router<S = ()> {
    routes: Vec<RouteSpec<S>>,
}

struct RouteSpec<S> {
    method: Method,
    path: Arc<str>,
    handler: HandlerFn<S>,
}

impl<S> Clone for RouteSpec<S> {
    fn clone(&self) -> Self {
        RouteSpec {
            method: self.method.clone(),
            path: self.path.clone(),
            handler: self.handler.clone(),
        }
    }
}

impl Router {
    pub(crate) fn new() -> Self {
        Self::with_state::<()>()
    }

    pub(crate) fn with_state<S>() -> Router<S> {
        Router { routes: Vec::new() }
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// A callable router-like type that can handle or pass on a request.
pub trait Callable<S>: Clone {
    /// Call with `state` and `request`, returning whether it was handled.
    fn call(&self, state: S, request: Request<Body>) -> CallResult<S>;
}

/// Outcome of invoking a `Callable` router.
pub enum CallResult<S> {
    /// The router handled the request and produced a response.
    Handled(Response<SendBody>),
    /// The router didn't match; returns the original state and request.
    Unhandled(S, Request<Body>),
}

impl<S> Router<S> {
    /// Finalize routes and compile the internal path matcher.
    pub fn build(self) -> Service<S> {
        let mut by_path: HashMap<Arc<str>, RouteId> = HashMap::new();
        let mut paths: Vec<PathEntry<S>> = Vec::new();

        for r in self.routes {
            let id = if let Some(i) = by_path.get(&r.path) {
                *i
            } else {
                let i = RouteId(paths.len());
                by_path.insert(r.path.clone(), i);
                paths.push(PathEntry {
                    id: i,
                    path: r.path.clone(),
                    methods: Vec::new(),
                });
                i
            };

            paths[id.0].methods.push((r.method, r.handler));
        }

        let mut trie = matchit::Router::new();
        for p in paths.iter() {
            // If invalid pattern, skip inserting to keep behavior predictable
            if trie.insert(p.path.as_ref(), p.id).is_err() {
                continue;
            }
        }

        Service::new(BuiltRouter { paths, trie })
    }

    /// Register a handler for a specific HTTP method and path.
    pub fn handle<'a, T, H: Handler<T, S> + Send + Sync + 'static>(
        mut self,
        method: Method,
        path: &'a str,
        handler: H,
    ) -> Self {
        let path: Arc<str> = Arc::from(path);
        let handler: HandlerFn<S> = {
            let h_arc = Arc::new(handler);
            Arc::new(move |state, req| {
                let h = (*h_arc).clone();
                Handler::<T, S>::call(h, state, req)
            })
        };
        self.routes.push(RouteSpec {
            method,
            path,
            handler,
        });
        self
    }

    /// Register a GET handler.
    pub fn get<'a, T, H: Handler<T, S> + Send + Sync + 'static>(
        self,
        path: &'a str,
        handler: H,
    ) -> Self {
        self.handle(Method::GET, path, handler)
    }

    /// Register a POST handler.
    pub fn post<'a, T, H: Handler<T, S> + Send + Sync + 'static>(
        self,
        path: &'a str,
        handler: H,
    ) -> Self {
        self.handle(Method::POST, path, handler)
    }

    /// Register a PUT handler.
    pub fn put<'a, T, H: Handler<T, S> + Send + Sync + 'static>(
        self,
        path: &'a str,
        handler: H,
    ) -> Self {
        self.handle(Method::PUT, path, handler)
    }

    /// Register a DELETE handler.
    pub fn delete<'a, T, H: Handler<T, S> + Send + Sync + 'static>(
        self,
        path: &'a str,
        handler: H,
    ) -> Self {
        self.handle(Method::DELETE, path, handler)
    }

    /// Register a HEAD handler.
    pub fn head<'a, T, H: Handler<T, S> + Send + Sync + 'static>(
        self,
        path: &'a str,
        handler: H,
    ) -> Self {
        self.handle(Method::HEAD, path, handler)
    }

    /// Register an OPTIONS handler.
    pub fn options<'a, T, H: Handler<T, S> + Send + Sync + 'static>(
        self,
        path: &'a str,
        handler: H,
    ) -> Self {
        self.handle(Method::OPTIONS, path, handler)
    }

    /// Register a CONNECT handler.
    pub fn connect<'a, T, H: Handler<T, S> + Send + Sync + 'static>(
        self,
        path: &'a str,
        handler: H,
    ) -> Self {
        self.handle(Method::CONNECT, path, handler)
    }

    /// Register a PATCH handler.
    pub fn patch<'a, T, H: Handler<T, S> + Send + Sync + 'static>(
        self,
        path: &'a str,
        handler: H,
    ) -> Self {
        self.handle(Method::PATCH, path, handler)
    }

    /// Register a TRACE handler.
    pub fn trace<'a, T, H: Handler<T, S> + Send + Sync + 'static>(
        self,
        path: &'a str,
        handler: H,
    ) -> Self {
        self.handle(Method::TRACE, path, handler)
    }
}

/// Compiled router that performs path and method matching.
pub(crate) struct BuiltRouter<S> {
    paths: Vec<PathEntry<S>>,
    trie: matchit::Router<RouteId>,
}

struct PathEntry<S> {
    id: RouteId,
    path: Arc<str>,
    methods: Vec<(Method, HandlerFn<S>)>,
}

impl<S> Callable<S> for BuiltRouter<S> {
    fn call(&self, state: S, request: Request<Body>) -> CallResult<S> {
        let path = request.uri().path();
        match self.trie.at(path) {
            Ok(m) => {
                let id = *m.value;
                // unwrap: index comes from our own trie values
                let entry = self.paths.get(id.0).expect("valid trie index");

                // Find handler for method
                let method = request.method();
                if let Some((_, handler)) = entry.methods.iter().find(|(mm, _)| mm == method) {
                    let resp = (handler)(state, request);
                    CallResult::Handled(resp)
                } else {
                    CallResult::Unhandled(state, request)
                }
            }
            Err(_) => CallResult::Unhandled(state, request),
        }
    }
}

impl<S> Clone for PathEntry<S> {
    fn clone(&self) -> Self {
        PathEntry {
            id: self.id.clone(),
            path: self.path.clone(),
            methods: self.methods.clone(),
        }
    }
}

impl<S> Clone for BuiltRouter<S> {
    fn clone(&self) -> Self {
        BuiltRouter {
            paths: self.paths.clone(),
            trie: self.trie.clone(),
        }
    }
}

impl<S> Clone for Router<S> {
    fn clone(&self) -> Self {
        Router {
            routes: self.routes.clone(),
        }
    }
}

/// Identifier for a unique path entry in the router.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct RouteId(usize);

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

        let _respone = router.call(
            &mut state,
            Request::builder().uri("/").body(Body::empty()).unwrap(),
        );

        is_send(router);
    }
}
