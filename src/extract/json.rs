use crate::Body;
use crate::http::Request;
use serde::de::DeserializeOwned;

use super::FromRequest;
use crate::{into_res::IntoResponse, SendBody};

/// JSON body extractor.
///
/// Deserializes the request body as JSON into `T` using `serde_json`.
///
/// This extractor consumes the request. It validates the `Content-Type` header
/// when present and accepts media types of `application/json` and
/// `application/*+json`.
pub struct Json<T, const MAX: u64 = 10_485_760>(pub T);

fn content_type_is_json(headers: &crate::http::HeaderMap) -> Result<bool, JsonRejection> {
    match headers.get(crate::http::header::CONTENT_TYPE) {
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

/// Error returned when JSON deserialization fails or the content type is invalid.
///
/// Returned as a `400 Bad Request`.
pub struct JsonRejection;

impl IntoResponse for JsonRejection {
    fn into_response(self) -> crate::http::Response<SendBody> {
        crate::http::Response::builder()
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

    #[derive(serde::Serialize, serde::Deserialize)]
    struct P { a: i32 }

    #[test]
    fn json_ok_with_content_type() {
        fn handler(_m: crate::http::Method, j: Json<P>) -> String { j.0.a.to_string() }

        let router = Service::router().post("/j", handler).build();

        let req = crate::http::Request::builder()
            .method("POST")
            .uri("/j")
            .version(crate::http::Version::HTTP_11)
            .header(
                crate::http::header::CONTENT_TYPE,
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
        fn handler(_m: crate::http::Method, j: Json<P>) -> String { j.0.a.to_string() }

        let router = Service::router().post("/j2", handler).build();

        let req = crate::http::Request::builder()
            .method("POST")
            .uri("/j2")
            .version(crate::http::Version::HTTP_11)
            .body(Body::builder().data(serde_json::to_vec(&P { a: 7 }).unwrap()))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "7");
    }

    #[test]
    fn json_reject_wrong_content_type() {
        fn handler(_m: crate::http::Method, _j: Json<P>) -> &'static str { "ok" }

        let router = Service::router().post("/j3", handler).build();

        let req = crate::http::Request::builder()
            .method("POST")
            .uri("/j3")
            .version(crate::http::Version::HTTP_11)
            .header(crate::http::header::CONTENT_TYPE, "text/plain")
            .body(Body::builder().data(b"{\"a\":1}".to_vec()))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 400);
    }

    #[test]
    fn json_reject_invalid_json() {
        fn handler(_m: crate::http::Method, _j: Json<P>) -> &'static str { "ok" }

        let router = Service::router().post("/j4", handler).build();

        let req = crate::http::Request::builder()
            .method("POST")
            .uri("/j4")
            .version(crate::http::Version::HTTP_11)
            .header(crate::http::header::CONTENT_TYPE, "application/json")
            .body(Body::builder().data(b"not-json".to_vec()))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 400);
    }

    #[test]
    fn json_limit_10kb_reject_large() {
        #[derive(serde::Deserialize, serde::Serialize)]
        struct Big { s: String }

        fn handler(_m: crate::http::Method, _j: Json<Big, { 10 * 1024 }>) -> &'static str { "ok" }

        let router = Service::router().post("/jl", handler).build();

        let big = Big { s: "a".repeat(12 * 1024) };
        let req = crate::http::Request::builder()
            .method("POST")
            .uri("/jl")
            .version(crate::http::Version::HTTP_11)
            .header(crate::http::header::CONTENT_TYPE, "application/json")
            .body(Body::builder().data(serde_json::to_vec(&big).unwrap()))
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 400);
    }
}


