use crate::http::request::Parts;

use super::{FromRequestParts, PathParams};
use crate::http;
use crate::{into_res::IntoResponse, SendBody};

/// Extract path parameters and deserialize into `T`.
///
/// Supports both named parameters into structs and positional parameters into
/// tuples, using the order they appear in the route pattern.
pub struct Path<T>(pub T);

impl<S, T> FromRequestParts<S> for Path<T>
where
    T: serde::de::DeserializeOwned,
{
    type Rejection = PathRejection;

    fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let Some(params) = parts.extensions.get::<PathParams>() else {
            return Err(PathRejection);
        };

        // Try struct-like (named) first
        let map = serde::de::value::MapDeserializer::<_, serde::de::value::Error>::new(
            params.0.clone().into_iter(),
        );
        let map_de = serde::de::value::MapAccessDeserializer::new(map);
        if let Ok(v) = T::deserialize(map_de) {
            return Ok(Path(v));
        }

        // Fall back to tuple-like (positional) using only values in order
        let seq = serde::de::value::SeqDeserializer::<_, serde::de::value::Error>::new(
            params.0.iter().map(|(_, v)| v.clone()),
        );
        let seq_de = serde::de::value::SeqAccessDeserializer::new(seq);
        T::deserialize(seq_de).map(Path).map_err(|_| PathRejection)
    }
}

/// Error returned when path parameter deserialization fails.
///
/// Returned as a `400 Bad Request`.
pub struct PathRejection;

impl IntoResponse for PathRejection {
    fn into_response(self) -> http::Response<SendBody> {
        http::Response::builder()
            .status(400)
            .body(SendBody::none())
            // unwrap: building a basic 400 response cannot fail
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::Path;
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
    fn path_named_struct() {
        #[derive(serde::Deserialize)]
        struct P {
            user: String,
            book: String,
        }

        fn handler(Path(p): Path<P>, _r: http::Request<Body>) -> String {
            format!("{}:{}", p.user, p.book)
        }

        let router = Service::router()
            .get("/u/{user}/books/{book}", handler)
            .build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/u/alice/books/xyz")
            .version(http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "alice:xyz");
    }

    #[test]
    fn path_positional_tuple() {
        fn handler(Path((a, b)): Path<(String, String)>, _r: http::Request<Body>) -> String {
            format!("{}-{}", a, b)
        }

        let router = Service::router().get("/p/{a}/{b}", handler).build();

        let req = http::Request::builder()
            .method("GET")
            .uri("/p/10/20")
            .version(http::Version::HTTP_11)
            .body(Body::empty())
            .unwrap();

        let resp = router.call((), req);
        assert_eq!(resp.status(), 200);
        assert_eq!(read_body_string(resp), "10-20");
    }
}
