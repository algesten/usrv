use crate::http::{Method, Request};
use crate::Body;

pub(crate) fn request_matcher(_request: &Request<Body>, _method: &Method, _path: &str) -> bool {
    true
}
