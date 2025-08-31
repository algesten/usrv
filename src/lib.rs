#![forbid(unsafe_code)]
#![warn(clippy::all)]
#![allow(mismatched_lifetime_syntaxes)]
// #![deny(missing_docs)]

#[macro_use]
extern crate log;

pub use ureq_proto::http;

mod from_req;
mod handler;
mod into_res;
mod matcher;
mod router;
mod send_body;
mod service;
mod util;

pub use from_req::{FromRequest, FromRequestParts};
pub use handler::Handler;
pub use into_res::{IntoResponse, NotFound};
pub use router::{MethodHandler, MethodRouter, Router};
pub use send_body::{IntoSendBody, SendBody};
pub use service::Service;

pub use from_req::Query;

pub struct Body;
