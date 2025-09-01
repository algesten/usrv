use crate::{Body};
use crate::http::Request;

use super::FromRequest;
use crate::{into_res::IntoResponse, SendBody};
use crate::http;

/// UTF-8 text body extractor with a size limit.
///
/// Reads the entire request body as a UTF-8 [`String`], enforcing a maximum of
/// `MAX` bytes. Invalid UTF-8 or bodies larger than the limit are rejected with
/// a `400 Bad Request`.
///
/// The default limit is 10MB.
pub struct Text<const MAX: u64 = 10_485_760>(pub String);

impl<S, const MAX: u64> FromRequest<S> for Text<MAX> {
    type Rejection = TextRejection;
    fn from_request(_state: &S, request: Request<Body>) -> Result<Self, Self::Rejection> {
        let (_parts, mut body) = request.into_parts();
        let text = body
            .with_config()
            .limit(MAX)
            .read_to_string()
            .map_err(|_| TextRejection)?;
        Ok(Text(text))
    }
}

/// Error returned when reading a text body fails.
///
/// Returned as a `400 Bad Request`.
pub struct TextRejection;

impl IntoResponse for TextRejection {
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
    use crate::{Body, Service, SendBody};

    fn read_body_string(mut resp: http::Response<SendBody>) -> String {
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
    fn text_ok() {
        fn handler(_m: http::Method, t: super::Text) -> String { t.0 }

        let router = Service::router().post("/t", handler).build();

        let req = http::Request::builder()
            .method("POST")
            .uri("/t")
            .version(http::Version::HTTP_11)
            .body(Body::builder().data("hello"))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "hello");
    }

    #[test]
    fn text_invalid_utf8_rejected() {
        fn handler(_m: http::Method, _t: super::Text) -> &'static str { "ok" }

        let router = Service::router().post("/ti", handler).build();

        let req = http::Request::builder()
            .method("POST")
            .uri("/ti")
            .version(http::Version::HTTP_11)
            .body(Body::builder().data(vec![0xFF, 0xFE, 0xFD]))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 400);
    }

    #[test]
    fn text_limit_rejected() {
        fn handler(_m: http::Method, _t: super::Text<4>) -> &'static str { "ok" }

        let router = Service::router().post("/tl", handler).build();

        let req = http::Request::builder()
            .method("POST")
            .uri("/tl")
            .version(http::Version::HTTP_11)
            .body(Body::builder().data("12345"))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 400);
    }
}


