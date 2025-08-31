//! Response conversion helpers.
//!
//! Types implementing [`IntoResponse`] can be returned from handlers and will
//! be converted into a [`http::Response`] with a [`SendBody`].
//!
//! # Example
//!
//! ```no_run
//! use usrv::Router;
//!
//! fn hello() -> &'static str { "hello" }
//!
//! let _router = Router::new().get("/hello", hello).build();
//! ```
use std::convert::Infallible;

use crate::http::Response;
use crate::{IntoSendBody, SendBody};

/// Convert a value into an HTTP response.
pub trait IntoResponse {
    /// Convert `self` into a [`http::Response`](crate::http::Response) with a [`SendBody`].
    ///
    /// Implemented for any type that implements [`IntoSendBody`], as well as
    /// specific response helper types in this crate.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use usrv::Router;
    ///
    /// fn hello() -> &'static str { "hello" }
    /// let _router = Router::new().get("/hello", hello).build();
    /// ```
    fn into_response(self) -> Response<SendBody>;
}

/// A 404 Not Found response.
///
/// Returned when no route matches.
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
