use crate::Body;
use crate::http::{request::Parts, Request};
use std::convert::Infallible;

use super::{FromRequest, FromRequestParts};

impl<S, T> FromRequestParts<S> for Option<T>
where
    T: FromRequestParts<S>,
{
    type Rejection = Infallible;
    fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(T::from_request_parts(parts, state).ok())
    }
}

impl<S, T> FromRequest<S> for Option<T>
where
    T: FromRequest<S>,
{
    type Rejection = Infallible;
    fn from_request(state: &S, request: Request<Body>) -> Result<Self, Self::Rejection> {
        Ok(T::from_request(state, request).ok())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Body, NotFound, Service, SendBody};

    #[derive(Clone)]
    struct XId(String);

    impl<S> super::FromRequestParts<S> for XId {
        type Rejection = NotFound;
        fn from_request_parts(parts: &mut crate::http::request::Parts, _state: &S) -> Result<Self, Self::Rejection> {
            let Some(v) = parts.headers.get("x-id").and_then(|h| h.to_str().ok()) else { return Err(NotFound) };
            Ok(XId(v.to_string()))
        }
    }

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
    fn option_wrapper_absent() {
        fn handler(_m: crate::http::Method, x: Option<XId>, _r: crate::http::Request<Body>) -> String {
            x.map(|x| x.0).unwrap_or_else(|| "none".to_string())
        }
        let router = Service::router().get("/opt", handler).build();

        let req = crate::http::Request::builder()
            .method("GET")
            .uri("/opt")
            .version(crate::http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "none");
    }

    #[test]
    fn option_wrapper_present() {
        fn handler(_m: crate::http::Method, x: Option<XId>, _r: crate::http::Request<Body>) -> String { x.unwrap().0 }
        let router = Service::router().get("/opt2", handler).build();

        let req = crate::http::Request::builder()
            .method("GET")
            .uri("/opt2")
            .version(crate::http::Version::HTTP_11)
            .header("x-id", "abc")
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "abc");
    }
}


