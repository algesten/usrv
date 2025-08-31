use std::marker::PhantomData;

use crate::http::{Request, Response};
use crate::router::{CallResult, Callable};
use crate::{Body, IntoResponse, NotFound, SendBody};

/// A callable service produced from a router.
///
/// Build using `Router::build`, then call with a state and request to
/// produce a response.
pub struct Service<S, R> {
    _state: PhantomData<S>,
    router: R,
}

impl<S, R: Callable<S>> Service<S, R> {
    pub(crate) fn new(router: R) -> Self {
        Service {
            _state: PhantomData,
            router,
        }
    }

    /// Call the service with the given state and request.
    pub fn call(&self, state: S, request: Request<Body>) -> Response<SendBody> {
        match self.router.call(state, request) {
            CallResult::Handled(v) => v,
            CallResult::Unhandled(_, _) => NotFound.into_response(),
        }
    }
}

impl<S, P: Clone> Clone for Service<S, P> {
    fn clone(&self) -> Self {
        Self {
            _state: PhantomData,
            router: self.router.clone(),
        }
    }
}
