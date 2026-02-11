//! Extractors for building handlers.
//!
//! This module provides two extractor traits:
//!
//! - [`FromRequestParts`] for head-only data (method, uri, headers, query, etc.).
//! - [`FromRequest`] for extractors that consume the full [`http::Request`](crate::http::Request).
//!
//! Extractors are used as handler function parameters. Leading parameters are
//! built from request parts, and the last parameter may consume the request.

use crate::http::{request::Parts, Request};
use crate::into_res::IntoResponse;
use crate::Body;

mod body;
mod bytes;
mod extension;
mod headers;
mod host;
#[cfg(feature = "json")]
mod json;
mod method;
mod option;
mod path;
#[cfg(feature = "query")]
mod query;
mod request;
mod result_;
mod text;
mod uri;
mod version;

pub use bytes::Bytes;
pub use extension::Extension;
pub use host::Host;
#[cfg(feature = "json")]
pub use json::Json;
pub use path::Path;
#[cfg(feature = "query")]
pub use query::{FromQuery, Query};
pub use text::Text;

/// Rejection types returned by extractors.
///
/// These types implement [`IntoResponse`] and are returned when an extractor
/// fails to build its value.
pub mod rejection {
    pub use super::bytes::BytesRejection;
    pub use super::extension::ExtensionRejection;
    #[cfg(feature = "json")]
    pub use super::json::JsonRejection;
    pub use super::path::PathRejection;
    #[cfg(feature = "query")]
    pub use super::query::QueryRejection;
    pub use super::text::TextRejection;
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
