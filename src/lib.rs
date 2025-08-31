//! Minimal, synchronous HTTP/1.1 server.
#![forbid(unsafe_code)]
#![warn(clippy::all)]
#![allow(mismatched_lifetime_syntaxes)]
#![deny(missing_docs)]

#[macro_use]
extern crate log;

/// Re-export of `http` types used throughout the public API.
#[doc(inline)]
pub use ureq_proto::http as http;

mod handler;
mod into_res;
mod matcher;
mod send_body;
mod service;
mod util;
mod error;

pub mod extract;
pub mod router;

pub use error::Error;

/// Convert a value into an HTTP response.
#[doc(inline)]
pub use into_res::IntoResponse;

/// A 404 Not Found response.
#[doc(inline)]
pub use into_res::NotFound;

/// Convert values into `SendBody`.
#[doc(inline)]
pub use send_body::IntoSendBody;

/// Streaming HTTP response body.
#[doc(inline)]
pub use send_body::SendBody;

/// A callable service produced from a router.
#[doc(inline)]
pub use service::Service;

/// Placeholder request body type used with `http::Request`.
pub struct Body;
