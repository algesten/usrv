use std::convert::Infallible;
// std collections used only in tests; see cfg(test) module below

use crate::http::{Request, Response};
use crate::into_res::IntoResponse;
use crate::{Body, SendBody};
use serde::de::DeserializeOwned;

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
    type Rejection: IntoResponse;
    fn from_request(state: &S, request: &Request<Body>) -> Result<Self, Self::Rejection>;
}

// Core non-consuming extractors mirroring axum-like parts
impl<S> RefArg<S> for crate::http::Method {
    type Rejection = Infallible;
    fn from_request(_state: &S, request: &Request<Body>) -> Result<Self, Self::Rejection> {
        Ok(request.method().clone())
    }
}

impl<S> RefArg<S> for crate::http::Uri {
    type Rejection = Infallible;
    fn from_request(_state: &S, request: &Request<Body>) -> Result<Self, Self::Rejection> {
        Ok(request.uri().clone())
    }
}

impl<S> RefArg<S> for crate::http::Version {
    type Rejection = Infallible;
    fn from_request(_state: &S, request: &Request<Body>) -> Result<Self, Self::Rejection> {
        Ok(request.version())
    }
}

impl<S> RefArg<S> for crate::http::HeaderMap {
    type Rejection = Infallible;
    fn from_request(_state: &S, request: &Request<Body>) -> Result<Self, Self::Rejection> {
        Ok(request.headers().clone())
    }
}

// Query extractor
pub struct Query<T>(pub T);

pub struct QueryRejection;

impl IntoResponse for QueryRejection {
    fn into_response(self) -> Response<SendBody> {
        Response::builder()
            .status(400)
            .body(SendBody::none())
            // unwrap: building a basic 400 response body cannot fail
            .unwrap()
    }
}

pub trait FromQuery: Sized {
    fn from_query(query: Option<&str>) -> Result<Self, QueryRejection>;
}

impl<T> FromQuery for T
where
    T: DeserializeOwned,
{
    fn from_query(query: Option<&str>) -> Result<Self, QueryRejection> {
        serde_urlencoded::from_str(query.unwrap_or("")).map_err(|_| QueryRejection)
    }
}

impl<S, T: FromQuery> RefArg<S> for Query<T> {
    type Rejection = QueryRejection;
    fn from_request(_state: &S, request: &Request<Body>) -> Result<Self, Self::Rejection> {
        let value = T::from_query(request.uri().query())?;
        Ok(Query(value))
    }
}

impl<S, T: FromQuery> Arg<S> for Query<T> {
    type Rejection = QueryRejection;
    fn from_request(_state: &S, request: Request<Body>) -> Result<Self, Self::Rejection> {
        let value = T::from_query(request.uri().query())?;
        Ok(Query(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http as http;
    use crate::{MethodRouter, Router};
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

        let svc = Router::new().get("/req", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/req")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = svc.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "ok");
    }

    #[test]
    fn extract_method() {
        fn handler(m: http::Method, _r: http::Request<Body>) -> String {
            // Echo the HTTP method
            m.to_string()
        }

        let svc = Router::new().get("/m", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/m")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = svc.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "GET");
    }

    #[test]
    fn extract_uri() {
        fn handler(u: http::Uri, _r: http::Request<Body>) -> String {
            // Echo the request path
            u.to_string()
        }

        let svc = Router::new().get("/u", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/u")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = svc.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "/u");
    }

    #[test]
    fn extract_version() {
        fn handler(v: http::Version, _r: http::Request<Body>) -> &'static str {
            // Indicate whether the request used HTTP/1.1
            if v == http::Version::HTTP_11 { "ok" } else { "bad" }
        }

        let svc = Router::new().get("/v", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/v")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = svc.call((), req);
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

        let svc = Router::new().get("/h", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/h")
            .version(http::Version::HTTP_11)
            .header("X-Unit", "ok")
            .body(Body)
            .unwrap();

        let resp = svc.call((), req);
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

        let svc = Router::new().get("/qv", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qv?b=2&a=1")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = svc.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "a=1&b=2");
    }

    #[test]
    fn extract_query_hashmap() {
        fn handler(q: Query<HashMap<String, String>>, _r: http::Request<Body>) -> String {
            // Build a deterministic output from two keys
            q.0.get("a").unwrap().to_string() + q.0.get("b").unwrap()
        }

        let svc = Router::new().get("/qh", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qh?a=1&b=2")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = svc.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "12");
    }

    #[test]
    fn extract_query_btreemap() {
        fn handler(q: Query<BTreeMap<String, String>>, _r: http::Request<Body>) -> String {
            // BTreeMap iteration is sorted by key
            q.0.iter().map(|(k,v)| format!("{k}={v}")).collect::<Vec<_>>().join(",")
        }

        let svc = Router::new().get("/qb", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qb?b=2&a=1")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = svc.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "a=1,b=2");
    }

    #[test]
    fn query_percent_encoded_not_decoded() {
        fn handler(q: Query<Vec<(String, String)>>, _r: http::Request<Body>) -> String {
            // Decode %xx encodings into UTF-8
            q.0.into_iter().map(|(k,v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")
        }

        let svc = Router::new().get("/qp", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qp?name=%20123")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = svc.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "name= 123");
    }

    #[test]
    fn query_plus_not_space() {
        fn handler(q: Query<Vec<(String, String)>>, _r: http::Request<Body>) -> String {
            // Axum semantics: '+' is treated as space in query strings
            q.0.into_iter().map(|(k,v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")
        }

        let svc = Router::new().get("/qplus", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/qplus?abc=a+b")
            .version(http::Version::HTTP_11)
            .body(Body)
            .unwrap();

        let resp = svc.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "abc=a b");
    }
}
