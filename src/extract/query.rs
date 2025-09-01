use crate::Body;
use crate::http::{request::Parts, Request};
use serde::de::DeserializeOwned;

use super::{FromRequest, FromRequestParts};
use crate::{into_res::IntoResponse, SendBody};
use crate::http;

/// Query string extractor.
///
/// Parses the URI query string using `application/x-www-form-urlencoded`
/// semantics (percent-decodes and treats `+` as space) and deserializes into
/// `T` via `serde_urlencoded`.
pub struct Query<T>(pub T);

/// Internal helper to construct `Query<T>` from a query string.
pub trait FromQuery: Sized {
    /// Parse an optional raw query string into `Self`.
    ///
    /// Uses `application/x-www-form-urlencoded` semantics (percent-decodes and
    /// treats `+` as space) and deserializes with `serde_urlencoded`.
    fn from_query(query: Option<&str>) -> Result<Self, QueryRejection>;
}

impl<T> FromQuery for T
where
    T: DeserializeOwned,
{
    fn from_query(query: Option<&str>) -> Result<Self, QueryRejection> {
        let s = query.unwrap_or("");
        let pairs = form_urlencoded::parse(s.as_bytes());
        let de = serde_urlencoded::Deserializer::new(pairs);
        serde_path_to_error::deserialize(de).map_err(|_| QueryRejection)
    }
}

impl<S, T: FromQuery> FromRequestParts<S> for Query<T> {
    type Rejection = QueryRejection;
    fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let value = T::from_query(parts.uri.query())?;
        Ok(Query(value))
    }
}

impl<S, T: FromQuery> FromRequest<S> for Query<T> {
    type Rejection = QueryRejection;
    fn from_request(_state: &S, request: Request<Body>) -> Result<Self, Self::Rejection> {
        let value = T::from_query(request.uri().query())?;
        Ok(Query(value))
    }
}

/// Error returned when query string deserialization fails.
///
/// Returned as a `400 Bad Request`.
pub struct QueryRejection;

impl IntoResponse for QueryRejection {
    fn into_response(self) -> http::Response<SendBody> {
        http::Response::builder()
            .status(400)
            .body(SendBody::none())
            // unwrap: building a basic 400 response body cannot fail
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Body, Service, SendBody};
    use std::collections::{BTreeMap, HashMap};

    fn read_body_string(mut resp: crate::http::Response<SendBody>) -> String {
        let mut bytes = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            let n = resp.body_mut().read(&mut buf).unwrap();
            if n == 0 { break; }
            bytes.extend_from_slice(&buf[..n]);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    #[test]
    fn extract_query_vec() {
        fn handler(q: Query<Vec<(String, String)>>, _r: crate::http::Request<Body>) -> String {
            let mut parts: Vec<String> = q.0.into_iter().map(|(k, v)| format!("{k}={v}")).collect();
            parts.sort();
            parts.join("&")
        }

        let router = Service::router().get("/qv", handler).build();

        let req = crate::http::Request::builder()
            .method("GET")
            .uri("/qv?b=2&a=1")
            .version(crate::http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "a=1&b=2");
    }

    #[test]
    fn extract_query_hashmap() {
        fn handler(q: Query<HashMap<String, String>>, _r: crate::http::Request<Body>) -> String {
            q.0.get("a").unwrap().to_string() + q.0.get("b").unwrap()
        }

        let router = Service::router().get("/qh", handler).build();

        let req = crate::http::Request::builder()
            .method("GET")
            .uri("/qh?a=1&b=2")
            .version(crate::http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "12");
    }

    #[test]
    fn extract_query_btreemap() {
        fn handler(q: Query<BTreeMap<String, String>>, _r: crate::http::Request<Body>) -> String {
            q.0.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(",")
        }

        let router = Service::router().get("/qb", handler).build();

        let req = crate::http::Request::builder()
            .method("GET")
            .uri("/qb?b=2&a=1")
            .version(crate::http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "a=1,b=2");
    }

    #[test]
    fn extract_single_named_query_param() {
        #[derive(serde::Deserialize)]
        struct OnlyFoo { foo: String }

        fn handler(q: Query<OnlyFoo>, _r: crate::http::Request<Body>) -> String { q.0.foo }

        let router = Service::router().get("/one", handler).build();

        let req = crate::http::Request::builder()
            .method("GET")
            .uri("/one?foo=bar&ignore=1")
            .version(crate::http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "bar");
    }

    #[test]
    fn query_percent_encoded_not_decoded() {
        fn handler(q: Query<Vec<(String, String)>>, _r: crate::http::Request<Body>) -> String {
            q.0.into_iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")
        }

        let router = Service::router().get("/qp", handler).build();

        let req = crate::http::Request::builder()
            .method("GET")
            .uri("/qp?name=%20123")
            .version(crate::http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "name= 123");
    }

    #[test]
    fn query_plus_not_space() {
        fn handler(q: Query<Vec<(String, String)>>, _r: crate::http::Request<Body>) -> String {
            q.0.into_iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")
        }

        let router = Service::router().get("/qplus", handler).build();

        let req = crate::http::Request::builder()
            .method("GET")
            .uri("/qplus?abc=a+b")
            .version(crate::http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "abc=a b");
    }
}


