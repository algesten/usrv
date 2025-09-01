use crate::http;
use crate::http::request::Parts;

use super::FromRequestParts;
use crate::{into_res::IntoResponse, SendBody};
/// Host extractor.
///
/// Resolves the host using the `Host` header when present, otherwise falls
/// back to the URI authority (if any). Returns the authority string
/// (potentially including a port), e.g. `"example.com"` or `"example.com:8080"`.
pub struct Host(pub String);

impl<S> FromRequestParts<S> for Host {
    type Rejection = HostRejection;

    fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(h) = parts.headers.get(http::header::HOST) {
            if let Ok(s) = h.to_str() {
                return Ok(Host(s.to_string()));
            }
        }

        if let Some(auth) = parts.uri.authority() {
            return Ok(Host(auth.as_str().to_string()));
        }

        Err(HostRejection)
    }
}

/// Error returned when no host can be determined.
///
/// Returned as `400 Bad Request`.
pub struct HostRejection;

impl IntoResponse for HostRejection {
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
    use crate::http;
    use crate::{Body, SendBody, Service};

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
    fn host_from_header() {
        fn handler(h: super::Host, _r: http::Request<Body>) -> String {
            h.0
        }

        let router = Service::router().get("/h", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/h")
            .version(http::Version::HTTP_11)
            .header(http::header::HOST, "example.com:8080")
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "example.com:8080");
    }

    #[test]
    fn host_from_uri_authority() {
        fn handler(h: super::Host, _r: http::Request<Body>) -> String {
            h.0
        }

        let router = Service::router().get("/u", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("http://example.org/u")
            .version(http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "example.org");
    }

    #[test]
    fn host_missing_rejected() {
        fn handler(_h: super::Host, _r: http::Request<Body>) -> &'static str {
            "ok"
        }

        let router = Service::router().get("/m", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/m")
            .version(http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 400);
    }
}
