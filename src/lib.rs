//! Minimal, synchronous HTTP/1.1 server.
#![forbid(unsafe_code)]
#![warn(clippy::all)]
#![allow(mismatched_lifetime_syntaxes)]
#![deny(missing_docs)]

#[macro_use]
extern crate log;

/// Re-export of `http` types used throughout the public API.
#[doc(inline)]
pub use ureq_proto::http;

mod from_req;
mod handler;
mod into_res;
mod matcher;
mod router;
mod send_body;
mod service;
mod util;

/// Build values from request parts (head-only).
pub use from_req::FromRequestParts;

/// Build values by consuming the full request.
#[doc(inline)]
pub use from_req::FromRequest;

/// A function/closure that can handle a request.
#[doc(inline)]
pub use handler::Handler;

/// Convert a value into an HTTP response.
#[doc(inline)]
pub use into_res::IntoResponse;

/// A 404 Not Found response.
#[doc(inline)]
pub use into_res::NotFound;

/// Router builder.
#[doc(inline)]
pub use router::Router;


/// Convert values into `SendBody`.
#[doc(inline)]
pub use send_body::IntoSendBody;

/// Streaming HTTP response body.
#[doc(inline)]
pub use send_body::SendBody;

/// A callable service produced from a router.
#[doc(inline)]
pub use service::Service;

/// Query-string extractor.
#[doc(inline)]
pub use from_req::Query;

/// Placeholder request body type used with `http::Request`.
///
/// Handlers can accept `http::Request<usrv::Body>` to access the request head,
/// and extractors that implement [`FromRequest`] may consume it.
///
/// # Example
///
/// ```no_run
/// use usrv::{http, Router};
///
/// fn echo(req: http::Request<usrv::Body>) -> String {
///     format!("path={}", req.uri().path())
/// }
///
/// let router = Router::new().get("/echo", echo).build();
/// let req = http::Request::builder().uri("/echo").body(usrv::Body).unwrap();
/// let _resp = router.call((), req);
/// ```
pub struct Body;
