use crate::http::request::Parts;

use super::FromRequestParts;
use crate::http;
use crate::{into_res::IntoResponse, SendBody};

/// Extract a clone of type `T` from request extensions.
///
/// Requires `T: Clone + Send + Sync + 'static`. Returns `500 Internal Server Error`
/// if the extension is missing, mirroring Axum semantics.
pub struct Extension<T>(pub T);

impl<S, T> FromRequestParts<S> for Extension<T>
where
    T: Clone + Send + Sync + 'static,
{
    type Rejection = ExtensionRejection;

    fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match parts.extensions.get::<T>() {
            Some(v) => Ok(Extension(v.clone())),
            None => Err(ExtensionRejection),
        }
    }
}

/// Returned when a required extension is missing.
///
/// Returned as `500 Internal Server Error`.
pub struct ExtensionRejection;

impl IntoResponse for ExtensionRejection {
    fn into_response(self) -> http::Response<SendBody> {
        http::Response::builder()
            .status(500)
            .body(SendBody::none())
            // unwrap: building a basic 500 response cannot fail
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
    fn extension_ok() {
        #[derive(Clone)]
        struct Ctx(&'static str);

        fn handler(
            super::Extension(ctx): super::Extension<Ctx>,
            _r: http::Request<Body>,
        ) -> String {
            ctx.0.to_string()
        }

        let router = Service::router().get("/e", handler).build();

        let mut req = http::Request::builder()
            .method("GET")
            .uri("/e")
            .version(http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();
        req.extensions_mut().insert(Ctx("ctx"));

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "ctx");
    }

    #[test]
    fn extension_missing_is_500() {
        #[derive(Clone)]
        struct Ctx;

        fn handler(_c: super::Extension<Ctx>, _r: http::Request<Body>) -> &'static str {
            "ok"
        }

        let router = Service::router().get("/e", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/e")
            .version(http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 500);
    }
}
