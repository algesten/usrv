use std::convert::Infallible;

use crate::http::Response;
use crate::{IntoSendBody, SendBody};

pub trait IntoResponse {
    fn into_response(self) -> Response<SendBody>;
}

pub struct NotFound;

impl IntoResponse for NotFound {
    fn into_response(self) -> Response<SendBody> {
        Response::builder()
            .status(404)
            .body(SendBody::none())
            .unwrap()
    }
}

impl IntoResponse for Infallible {
    fn into_response(self) -> Response<SendBody> {
        panic!("IntoResponse for Infallible");
    }
}

impl<T> IntoResponse for T
where
    T: IntoSendBody,
{
    fn into_response(self) -> Response<SendBody> {
        Response::builder().body(self.into_body()).unwrap()
    }
}
