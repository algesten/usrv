use crate::Body;
use crate::http::Request;
use std::convert::Infallible;

use super::FromRequest;

impl<S> FromRequest<S> for Request<Body> {
    type Rejection = Infallible;
    fn from_request(_state: &S, request: Request<Body>) -> Result<Self, Self::Rejection> {
        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Body, Service, SendBody};

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
    fn extract_request_arg() {
        fn handler(_: crate::http::Request<Body>) -> &'static str { "ok" }

        let router = Service::router().get("/req", handler).build();

        let req = crate::http::Request::builder()
            .method("GET")
            .uri("/req")
            .version(crate::http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "ok");
    }
}


