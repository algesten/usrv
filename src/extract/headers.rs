use crate::http;
use crate::http::request::Parts;
use std::convert::Infallible;

use super::FromRequestParts;

/// Extract a clone of the header map from request parts.
impl<S> FromRequestParts<S> for http::HeaderMap {
    type Rejection = Infallible;
    fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.headers.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::http;
    use crate::{Body, Service};

    fn read_body_string(mut resp: http::Response<crate::SendBody>) -> String {
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
    fn extract_headers() {
        fn handler(h: http::HeaderMap, _r: http::Request<Body>) -> String {
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
}
