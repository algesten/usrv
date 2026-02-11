use crate::http::Request;
use crate::Body;

use super::FromRequest;
use crate::http;
use crate::{into_res::IntoResponse, SendBody};

/// Raw bytes body extractor with a size limit.
///
/// Reads the entire request body into a [`Vec<u8>`], enforcing a maximum of
/// `MAX` bytes. Bodies larger than the limit are rejected with a
/// `400 Bad Request`.
///
/// The default limit is 10MB.
pub struct Bytes<const MAX: u64 = 10_485_760>(pub Vec<u8>);

impl<S, const MAX: u64> FromRequest<S> for Bytes<MAX> {
    type Rejection = BytesRejection;
    fn from_request(_state: &S, request: Request<Body>) -> Result<Self, Self::Rejection> {
        let (_parts, mut body) = request.into_parts();
        let bytes = body
            .with_config()
            .limit(MAX)
            .read_to_vec()
            .map_err(|_| BytesRejection)?;
        Ok(Bytes(bytes))
    }
}

/// Error returned when reading a bytes body fails.
///
/// Returned as a `400 Bad Request`.
pub struct BytesRejection;

impl IntoResponse for BytesRejection {
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
    fn bytes_ok() {
        fn handler(_m: http::Method, b: super::Bytes) -> String {
            b.0.len().to_string()
        }

        let router = Service::router().post("/b", handler).build();

        let req = http::Request::builder()
            .method("POST")
            .uri("/b")
            .version(http::Version::HTTP_11)
            .body(Body::builder().data(vec![1, 2, 3, 4]))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "4");
    }

    #[test]
    fn bytes_limit_rejected() {
        fn handler(_m: http::Method, _b: super::Bytes<2>) -> &'static str {
            "ok"
        }

        let router = Service::router().post("/bl", handler).build();

        let req = http::Request::builder()
            .method("POST")
            .uri("/bl")
            .version(http::Version::HTTP_11)
            .body(Body::builder().data(vec![1, 2, 3]))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 400);
    }
}
