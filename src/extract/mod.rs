//! Extractors for building handlers.
//!
//! This module provides two extractor traits:
//!
//! - [`FromRequestParts`] for head-only data (method, uri, headers, query, etc.).
//! - [`FromRequest`] for extractors that consume the full [`http::Request`](crate::http::Request).
//!
//! Extractors are used as handler function parameters. Leading parameters are
//! built from request parts, and the last parameter may consume the request.

use crate::into_res::IntoResponse;
use crate::Body;
use crate::http::{request::Parts, Request};

mod method;
mod uri;
mod version;
mod headers;
mod request;
mod path;
mod option;
mod result_;
mod text;
mod bytes;
#[cfg(feature = "query")]
mod query;
#[cfg(feature = "json")]
mod json;

pub use bytes::Bytes;
#[cfg(feature = "json")]
pub use json::Json;
pub use path::Path;
pub use text::Text;
#[cfg(feature = "query")]
pub use query::{FromQuery, Query};

/// Rejection types returned by extractors.
///
/// These types implement [`IntoResponse`] and are returned when an extractor
/// fails to build its value.
pub mod rejection {
    pub use super::bytes::BytesRejection;
    #[cfg(feature = "json")]
    pub use super::json::JsonRejection;
    pub use super::path::PathRejection;
    pub use super::text::TextRejection;
    #[cfg(feature = "query")]
    pub use super::query::QueryRejection;
}

/// Builds a value from request head/parts.
///
/// Implementors can extract method, uri, headers, query parameters, and other
/// metadata that resides in the request head. The body is not accessible here.
///
/// See the crate-level example for typical usage.
pub trait FromRequestParts<S>: Sized {
    /// Error type produced when extraction fails.
    ///
    /// This is converted into a response via [`IntoResponse`].
    type Rejection: IntoResponse;

    /// Build `Self` from request head/parts and application state.
    ///
    /// This does not have access to the request body.
    fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection>;
}

/// Builds a value by consuming the full request.
///
/// Implementors may read the request body. This is used for body-based
/// extractors and for taking ownership of the request.
///
/// See the crate-level example for typical usage.
pub trait FromRequest<S>: Sized {
    /// Error type produced when extraction fails.
    ///
    /// This is converted into a response via [`IntoResponse`].
    type Rejection: IntoResponse;

    /// Build `Self` by consuming the full request.
    ///
    /// This is typically used for body-based extractors or to take ownership of
    /// the request as the last handler parameter.
    fn from_request(state: &S, request: Request<Body>) -> Result<Self, Self::Rejection>;
}

/// Matched path parameters stored on the request.
///
/// Inserted by the router upon a successful path match.
#[derive(Clone)]
pub(crate) struct PathParams(pub Vec<(String, String)>);



pub(crate) fn prepare_extracters<X>(m: &matchit::Match<X>, request: &mut Request<Body>) {
    let params: Vec<(String, String)> = m
        .params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    request.extensions_mut().insert(PathParams(params));
}


