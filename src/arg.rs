use std::convert::Infallible;

use crate::http::{Request, Response};
use crate::into_res::IntoResponse;
use crate::{Body, SendBody};

pub trait Arg<S>: Sized {
    type Rejection: IntoResponse;
    fn from_request(state: &S, request: Request<Body>) -> Result<Self, Self::Rejection>;
}

impl<S> Arg<S> for Request<Body> {
    type Rejection = Infallible;

    fn from_request(_state: &S, request: Request<Body>) -> Result<Self, Self::Rejection> {
        Ok(request)
    }
}

pub trait RefArg<S>: Sized {
    type Rejection: Into<Response<SendBody>>;
    fn from_request(state: &S, request: &Request<Body>) -> Result<Self, Self::Rejection>;
}
