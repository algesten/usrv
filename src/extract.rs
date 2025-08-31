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
//! use usrv::{http, Service, Body};
//! use usrv::extract::Query;
//!
//! #[derive(serde::Deserialize)]
//! struct Params { foo: String }
//!
//! fn handler(p: Query<Params>, _req: http::Request<Body>) -> String {
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
//!     .body(Body::empty())
//!     .unwrap();
//!
//! let _resp = router.call((), req);
//! # }
//! ```
use crate::http;
use crate::into_res::IntoResponse;
use crate::Body;
#[cfg(any(feature = "query", feature = "json"))]
use crate::{http::Response, SendBody};
use http::{request::Parts, Request};
#[cfg(any(feature = "query", feature = "json"))]
use serde::de::DeserializeOwned;
use std::convert::Infallible;

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
    /// use usrv::{http, Service, Body};
    /// use usrv::extract::FromRequestParts;
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
    /// fn handler(x: XId, _r: http::Request<Body>) -> String { x.0 }
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
    /// use usrv::{http, Service, Body};
    ///
    /// fn take(req: http::Request<Body>) -> String {
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

/// JSON body extractor.
///
/// Deserializes the request body as JSON into `T` using `serde_json`.
///
/// This extractor consumes the request. It validates the `Content-Type` header
/// when present and accepts media types of `application/json` and
/// `application/*+json`.
#[cfg(feature = "json")]
pub struct Json<T, const MAX: u64 = 10_485_760>(pub T);

/// Error returned when JSON deserialization fails or the content type is invalid.
///
/// Returned as a `400 Bad Request`.
#[cfg(feature = "json")]
pub struct JsonRejection;

#[cfg(feature = "json")]
impl IntoResponse for JsonRejection {
    fn into_response(self) -> Response<SendBody> {
        Response::builder()
            .status(400)
            .body(SendBody::none())
            // unwrap: building a basic 400 response body cannot fail
            .unwrap()
    }
}

#[cfg(feature = "json")]
fn content_type_is_json(headers: &http::HeaderMap) -> Result<bool, JsonRejection> {
    match headers.get(http::header::CONTENT_TYPE) {
        None => Ok(true),
        Some(value) => {
            let Ok(s) = value.to_str() else {
                return Err(JsonRejection);
            };
            let s = s.trim();
            // Split off parameters like "; charset=utf-8"
            let ty = s.split(';').next().unwrap();
            let ty = ty.trim().to_ascii_lowercase();

            if ty == "application/json" {
                return Ok(true);
            }
            if let Some(idx) = ty.rfind('+') {
                if &ty[..idx] != "application/" && !ty.starts_with("application/") {
                    return Err(JsonRejection);
                }
                if &ty[idx..] == "+json" {
                    return Ok(true);
                }
            }
            Err(JsonRejection)
        }
    }
}

#[cfg(feature = "json")]
impl<S, T, const MAX: u64> FromRequest<S> for Json<T, MAX>
where
    T: DeserializeOwned,
{
    type Rejection = JsonRejection;
    fn from_request(_state: &S, request: Request<Body>) -> Result<Self, Self::Rejection> {
        if !content_type_is_json(request.headers())? {
            return Err(JsonRejection);
        }

        let (_parts, mut body) = request.into_parts();
        let reader = body.with_config().limit(MAX).reader();
        let value: T = serde_json::from_reader(reader).map_err(|_| JsonRejection)?;
        Ok(Json(value))
    }
}

#[cfg(all(test, feature = "query"))]
mod tests {
    use super::*;
    use crate::Service;
    use std::collections::{BTreeMap, HashMap};

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
            .body(Body::empty())
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
            .body(Body::empty())
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
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "/u");
    }

    #[test]
    fn extract_version() {
        fn handler(v: http::Version, _r: http::Request<Body>) -> &'static str {
            // Indicate whether the request used HTTP/1.1
            if v == http::Version::HTTP_11 {
                "ok"
            } else {
                "bad"
            }
        }

        let router = Service::router().get("/v", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/v")
            .version(http::Version::HTTP_11)
            .body(Body::empty())
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
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "ok");
    }

    #[test]
    fn extract_query_vec() {
        fn handler(q: Query<Vec<(String, String)>>, _r: http::Request<Body>) -> String {
            // Collect and sort pairs to make the output stable for testing
            let mut parts: Vec<String> = q.0.into_iter().map(|(k, v)| format!("{k}={v}")).collect();
            parts.sort();
            parts.join("&")
        }

        let router = Service::router().get("/qv", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qv?b=2&a=1")
            .version(http::Version::HTTP_11)
            .body(Body::empty())
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
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "12");
    }

    #[test]
    fn extract_query_btreemap() {
        fn handler(q: Query<BTreeMap<String, String>>, _r: http::Request<Body>) -> String {
            // BTreeMap iteration is sorted by key
            q.0.iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(",")
        }

        let router = Service::router().get("/qb", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qb?b=2&a=1")
            .version(http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "a=1,b=2");
    }

    #[test]
    fn extract_single_named_query_param() {
        #[derive(serde::Deserialize)]
        struct OnlyFoo {
            foo: String,
        }

        fn handler(q: Query<OnlyFoo>, _r: http::Request<Body>) -> String {
            q.0.foo
        }

        let router = Service::router().get("/one", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/one?foo=bar&ignore=1")
            .version(http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "bar");
    }

    #[test]
    fn query_percent_encoded_not_decoded() {
        fn handler(q: Query<Vec<(String, String)>>, _r: http::Request<Body>) -> String {
            // Decode %xx encodings into UTF-8
            q.0.into_iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&")
        }

        let router = Service::router().get("/qp", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qp?name=%20123")
            .version(http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "name= 123");
    }

    #[test]
    fn query_plus_not_space() {
        fn handler(q: Query<Vec<(String, String)>>, _r: http::Request<Body>) -> String {
            // Query semantics: '+' is treated as space in query strings
            q.0.into_iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&")
        }

        let router = Service::router().get("/qplus", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qplus?abc=a+b")
            .version(http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "abc=a b");
    }
}

#[cfg(all(test, feature = "json"))]
mod json_tests {
    use super::*;
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

    #[derive(serde::Serialize, serde::Deserialize)]
    struct P {
        a: i32,
    }

    #[test]
    fn json_ok_with_content_type() {
        #[cfg(feature = "json")]
        fn handler(_m: http::Method, j: Json<P>) -> String {
            j.0.a.to_string()
        }

        let router = Service::router().post("/j", handler).build();

        let req = http::Request::builder()
            .method("POST")
            .uri("/j")
            .version(http::Version::HTTP_11)
            .header(
                http::header::CONTENT_TYPE,
                "application/json; charset=utf-8",
            )
            .body(Body::builder().data(serde_json::to_vec(&P { a: 42 }).unwrap()))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "42");
    }

    #[test]
    fn json_ok_without_content_type() {
        #[cfg(feature = "json")]
        fn handler(_m: http::Method, j: Json<P>) -> String {
            j.0.a.to_string()
        }

        let router = Service::router().post("/j2", handler).build();

        let req = http::Request::builder()
            .method("POST")
            .uri("/j2")
            .version(http::Version::HTTP_11)
            .body(Body::builder().data(serde_json::to_vec(&P { a: 7 }).unwrap()))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "7");
    }

    #[test]
    fn json_reject_wrong_content_type() {
        #[cfg(feature = "json")]
        fn handler(_m: http::Method, _j: Json<P>) -> &'static str {
            "ok"
        }

        let router = Service::router().post("/j3", handler).build();

        let req = http::Request::builder()
            .method("POST")
            .uri("/j3")
            .version(http::Version::HTTP_11)
            .header(http::header::CONTENT_TYPE, "text/plain")
            .body(Body::builder().data(b"{\"a\":1}".to_vec()))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 400);
    }

    #[test]
    fn json_reject_invalid_json() {
        #[cfg(feature = "json")]
        fn handler(_m: http::Method, _j: Json<P>) -> &'static str {
            "ok"
        }

        let router = Service::router().post("/j4", handler).build();

        let req = http::Request::builder()
            .method("POST")
            .uri("/j4")
            .version(http::Version::HTTP_11)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::builder().data(b"not-json".to_vec()))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 400);
    }

    #[test]
    fn json_limit_10kb_reject_large() {
        #[derive(serde::Deserialize, serde::Serialize)]
        struct Big { s: String }

        #[cfg(feature = "json")]
        fn handler(_m: http::Method, _j: Json<Big, { 10 * 1024 }>) -> &'static str { "ok" }

        let router = Service::router().post("/jl", handler).build();

        let big = Big { s: "a".repeat(12 * 1024) };
        let req = http::Request::builder()
            .method("POST")
            .uri("/jl")
            .version(http::Version::HTTP_11)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::builder().data(serde_json::to_vec(&big).unwrap()))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 400);
    }
}
