use std::convert::Infallible;

use crate::http::Request;
use crate::Body;

use super::FromRequest;

impl<S> FromRequest<S> for Body {
    type Rejection = Infallible;

    fn from_request(_state: &S, request: Request<Body>) -> Result<Self, Self::Rejection> {
        let (_parts, body) = request.into_parts();
        Ok(body)
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
    fn extract_body_ok() {
        fn handler(_m: http::Method, mut b: Body) -> String {
            b.read_to_string().unwrap()
        }

        let router = Service::router().post("/body", handler).build();

        let req = http::Request::builder()
            .method("POST")
            .uri("/body")
            .version(http::Version::HTTP_11)
            .body(Body::builder().data("hello"))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "hello");
    }
}
