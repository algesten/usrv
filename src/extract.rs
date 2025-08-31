//! Extractors for building handlers.
//!
//! This module provides two extractor traits:
//!
//! - [`FromRequestParts`] for head-only data (method, uri, headers, query, etc.).
//! - [`FromRequest`] for extractors that consume the full [`http::Request`].
//!
//! Extractors are used as handler function parameters. Leading parameters are
//! built from request parts, and the last parameter may consume the request.
//!
//! # Example
//!
//! ```no_run
//! # #[cfg(feature = "query")]
//! # {
//! use usrv::{http, Service};
//! use usrv::extract::Query;
//!
//! #[derive(serde::Deserialize)]
//! struct Params { foo: String }
//!
//! fn handler(p: Query<Params>, _req: http::Request<usrv::Body>) -> String {
//!     format!("foo={}", p.0.foo)
//! }
//!
//! let router = Service::router()
//!     .get("/hello", handler)
//!     .build();
//!
//! let req = http::Request::builder()
//!     .method("GET")
//!     .uri("/hello?foo=bar")
//!     .body(usrv::Body)
//!     .unwrap();
//!
//! let _resp = router.call((), req);
//! # }
//! ```
use std::convert::Infallible;
use crate::http;
use http::{request::Parts, Request};
use crate::into_res::IntoResponse;
use crate::Body;
#[cfg(feature = "query")]
use crate::{http::Response, SendBody};
#[cfg(feature = "query")]
use serde::de::DeserializeOwned;

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
    ///
    /// # Example
    ///
    /// ```no_run
    /// use usrv::http;
    /// use usrv::extract::FromRequestParts;
    /// use usrv::Service;
    ///
    /// #[derive(Clone)]
    /// struct XId(String);
    ///
    /// impl<S> FromRequestParts<S> for XId {
    ///     type Rejection = usrv::NotFound;
    ///     fn from_request_parts(parts: &mut http::request::Parts, _state: &S) -> Result<Self, Self::Rejection> {
    ///         let v = parts.headers.get("x-id").and_then(|h| h.to_str().ok()).ok_or(usrv::NotFound)?;
    ///         Ok(XId(v.to_string()))
    ///     }
    /// }
    ///
    /// fn handler(x: XId, _r: http::Request<usrv::Body>) -> String { x.0 }
    /// let _router = Service::router().get("/", handler).build();
    /// ```
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// use usrv::{http, Service};
    ///
    /// fn take(req: http::Request<usrv::Body>) -> String {
    ///     req.uri().to_string()
    /// }
    ///
    /// let _router = Service::router().get("/path", take).build();
    /// ```
    fn from_request(state: &S, request: Request<Body>) -> Result<Self, Self::Rejection>;
}

impl<S> FromRequest<S> for Request<Body> {
    type Rejection = Infallible;
    fn from_request(_state: &S, request: Request<Body>) -> Result<Self, Self::Rejection> {
        Ok(request)
    }
}

/// Extract the HTTP method from request parts.
impl<S> FromRequestParts<S> for http::Method {
    type Rejection = Infallible;
    fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.method.clone())
    }
}

/// Extract the request URI from request parts.
impl<S> FromRequestParts<S> for http::Uri {
    type Rejection = Infallible;
    fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.uri.clone())
    }
}

/// Extract the HTTP version from request parts.
impl<S> FromRequestParts<S> for http::Version {
    type Rejection = Infallible;
    fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.version)
    }
}

/// Extract a clone of the header map from request parts.
impl<S> FromRequestParts<S> for http::HeaderMap {
    type Rejection = Infallible;
    fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.headers.clone())
    }
}

/// Query string extractor.
///
/// Parses the URI query string using `application/x-www-form-urlencoded`
/// semantics (percent-decodes and treats `+` as space) and deserializes into
/// `T` via `serde_urlencoded`.
#[cfg(feature = "query")]
pub struct Query<T>(pub T);

/// Error returned when query string deserialization fails.
///
/// Returned as a `400 Bad Request`.
#[cfg(feature = "query")]
pub struct QueryRejection;

#[cfg(feature = "query")]
impl IntoResponse for QueryRejection {
    fn into_response(self) -> Response<SendBody> {
        Response::builder()
            .status(400)
            .body(SendBody::none())
            // unwrap: building a basic 400 response body cannot fail
            .unwrap()
    }
}

/// Internal helper to construct `Query<T>` from a query string.
#[cfg(feature = "query")]
pub trait FromQuery: Sized {
    /// Parse an optional raw query string into `Self`.
    ///
    /// Uses `application/x-www-form-urlencoded` semantics (percent-decodes and
    /// treats `+` as space) and deserializes with `serde_urlencoded`.
    fn from_query(query: Option<&str>) -> Result<Self, QueryRejection>;
}

#[cfg(feature = "query")]
impl<T> FromQuery for T
where
    T: DeserializeOwned,
{
    fn from_query(query: Option<&str>) -> Result<Self, QueryRejection> {
        serde_urlencoded::from_str(query.unwrap_or("")).map_err(|_| QueryRejection)
    }
}

#[cfg(feature = "query")]
impl<S, T: FromQuery> FromRequestParts<S> for Query<T> {
    type Rejection = QueryRejection;
    fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let value = T::from_query(parts.uri.query())?;
        Ok(Query(value))
    }
}

#[cfg(feature = "query")]
impl<S, T: FromQuery> FromRequest<S> for Query<T> {
    type Rejection = QueryRejection;
    fn from_request(_state: &S, request: Request<Body>) -> Result<Self, Self::Rejection> {
        let value = T::from_query(request.uri().query())?;
        Ok(Query(value))
    }
}

#[cfg(all(test, feature = "query"))]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashMap};
    use crate::Service;

    fn read_body_string(mut resp: http::Response<SendBody>) -> String {
        let mut bytes = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            let n = resp.body_mut().read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..n]);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    #[test]
    fn extract_request_arg() {
        fn handler(_: http::Request<Body>) -> &'static str {
            // Return a simple body string so users see the end-to-end flow
            "ok"
        }

        let router = Service::router().get("/req", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/req")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "ok");
    }

    #[test]
    fn extract_method() {
        fn handler(m: http::Method, _r: http::Request<Body>) -> String {
            // Echo the HTTP method
            m.to_string()
        }

        let router = Service::router().get("/m", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/m")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "GET");
    }

    #[test]
    fn extract_uri() {
        fn handler(u: http::Uri, _r: http::Request<Body>) -> String {
            // Echo the request path
            u.to_string()
        }

        let router = Service::router().get("/u", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/u")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "/u");
    }

    #[test]
    fn extract_version() {
        fn handler(v: http::Version, _r: http::Request<Body>) -> &'static str {
            // Indicate whether the request used HTTP/1.1
            if v == http::Version::HTTP_11 { "ok" } else { "bad" }
        }

        let router = Service::router().get("/v", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/v")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "ok");
    }

    #[test]
    fn extract_headers() {
        fn handler(h: http::HeaderMap, _r: http::Request<Body>) -> String {
            // Read a custom header and default to empty
            h.get("X-Unit")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string()
        }

        let router = Service::router().get("/h", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/h")
            .version(http::Version::HTTP_11)
            .header("X-Unit", "ok")
            .body(Body)
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "ok");
    }

    #[test]
    fn extract_query_vec() {
        fn handler(q: Query<Vec<(String, String)>>, _r: http::Request<Body>) -> String {
            // Collect and sort pairs to make the output stable for testing
            let mut parts: Vec<String> = q.0.into_iter().map(|(k,v)| format!("{k}={v}")).collect();
            parts.sort();
            parts.join("&")
        }

        let router = Service::router().get("/qv", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qv?b=2&a=1")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "a=1&b=2");
    }

    #[test]
    fn extract_query_hashmap() {
        fn handler(q: Query<HashMap<String, String>>, _r: http::Request<Body>) -> String {
            // Build a deterministic output from two keys
            q.0.get("a").unwrap().to_string() + q.0.get("b").unwrap()
        }

        let router = Service::router().get("/qh", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qh?a=1&b=2")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "12");
    }

    #[test]
    fn extract_query_btreemap() {
        fn handler(q: Query<BTreeMap<String, String>>, _r: http::Request<Body>) -> String {
            // BTreeMap iteration is sorted by key
            q.0.iter().map(|(k,v)| format!("{k}={v}")).collect::<Vec<_>>().join(",")
        }

        let router = Service::router().get("/qb", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qb?b=2&a=1")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "a=1,b=2");
    }

    #[test]
    fn extract_single_named_query_param() {
        #[derive(serde::Deserialize)]
        struct OnlyFoo { foo: String }

        fn handler(q: Query<OnlyFoo>, _r: http::Request<Body>) -> String {
            q.0.foo
        }

        let router = Service::router().get("/one", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/one?foo=bar&ignore=1")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "bar");
    }

    #[test]
    fn query_percent_encoded_not_decoded() {
        fn handler(q: Query<Vec<(String, String)>>, _r: http::Request<Body>) -> String {
            // Decode %xx encodings into UTF-8
            q.0.into_iter().map(|(k,v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")
        }

        let router = Service::router().get("/qp", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qp?name=%20123")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "name= 123");
    }

    #[test]
    fn query_plus_not_space() {
        fn handler(q: Query<Vec<(String, String)>>, _r: http::Request<Body>) -> String {
            // Query semantics: '+' is treated as space in query strings
            q.0.into_iter().map(|(k,v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")
        }

        let router = Service::router().get("/qplus", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qplus?abc=a+b")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "abc=a b");
    }
}
