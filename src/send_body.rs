use std::fs::File;
use std::io::{self, Read, Stdin};
use std::net::TcpStream;

#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;

use crate::util::private::Private;

// MSRV 1.78
// impl_into_body!(&Stdin, Reader);

pub struct SendBody {
    inner: BodyInner,
    ended: bool,
}

impl SendBody {
    /// Creates an empty body.
    pub fn none() -> SendBody {
        BodyInner::None.into()
    }

    /// Creates a body from an owned [`Read]` impl.
    pub fn from_owned_reader(reader: impl Read + 'static) -> SendBody {
        BodyInner::OwnedReader(Box::new(reader)).into()
    }

    /// Creates a body to send as JSON from any [`Serialize`](serde::ser::Serialize) value.
    #[cfg(feature = "json")]
    pub fn from_json(value: &impl serde::ser::Serialize) -> Result<SendBody, crate::Error> {
        let json = serde_json::to_vec_pretty(value)?;
        Ok(BodyInner::ByteVec(io::Cursor::new(json)).into())
    }

    pub(crate) fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.ended {
            return Ok(0);
        }

        let n = match &mut self.inner {
            BodyInner::None => {
                return Ok(0);
            }
            BodyInner::ByteVec(v) => v.read(buf),
            BodyInner::OwnedReader(v) => v.read(buf),
        }?;

        if n == 0 {
            trace!("SendBody ended");
            self.ended = true;
        }

        Ok(n)
    }

    // pub(crate) fn body_mode(&self) -> BodyMode {
    //     self.inner.body_mode()
    // }

    pub fn into_reader(self) -> impl Sized + io::Read {
        ReadAdapter(self)
    }
}

struct ReadAdapter(SendBody);

impl io::Read for ReadAdapter {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

pub trait IntoSendBody: Private {
    #[doc(hidden)]
    fn into_body(self) -> SendBody;
}

pub(crate) enum BodyInner {
    None,
    ByteVec(io::Cursor<Vec<u8>>),
    OwnedReader(Box<dyn Read>),
}

// impl BodyInner {
//     pub fn body_mode(&self) -> BodyMode {
//         match self {
//             BodyInner::None => BodyMode::NoBody,
//             BodyInner::ByteVec(v) => BodyMode::LengthDelimited(v.get_ref().len() as u64),
//             BodyInner::OwnedReader(_) => BodyMode::Chunked,
//         }
//     }
// }

impl Private for &[u8] {}
impl IntoSendBody for &[u8] {
    fn into_body(self) -> SendBody {
        BodyInner::ByteVec(io::Cursor::new(self.to_vec())).into()
    }
}

impl Private for &str {}
impl IntoSendBody for &str {
    fn into_body(self) -> SendBody {
        BodyInner::ByteVec(io::Cursor::new(self.as_bytes().to_vec())).into()
    }
}

impl Private for String {}
impl IntoSendBody for String {
    fn into_body(self) -> SendBody {
        BodyInner::ByteVec(io::Cursor::new(self.into_bytes())).into()
    }
}

impl Private for Vec<u8> {}
impl IntoSendBody for Vec<u8> {
    fn into_body(self) -> SendBody {
        BodyInner::ByteVec(io::Cursor::new(self)).into()
    }
}

impl Private for &String {}
impl IntoSendBody for &String {
    fn into_body(self) -> SendBody {
        BodyInner::ByteVec(io::Cursor::new(self.as_bytes().to_vec())).into()
    }
}

impl Private for &Vec<u8> {}
impl IntoSendBody for &Vec<u8> {
    fn into_body(self) -> SendBody {
        BodyInner::ByteVec(io::Cursor::new(self.to_vec())).into()
    }
}

impl Private for File {}
impl IntoSendBody for File {
    fn into_body(self) -> SendBody {
        SendBody::from_owned_reader(Box::new(self))
    }
}

impl Private for TcpStream {}
impl IntoSendBody for TcpStream {
    fn into_body(self) -> SendBody {
        SendBody::from_owned_reader(Box::new(self))
    }
}

impl Private for Stdin {}
impl IntoSendBody for Stdin {
    fn into_body(self) -> SendBody {
        SendBody::from_owned_reader(Box::new(self))
    }
}

#[cfg(target_family = "unix")]
impl Private for UnixStream {}
#[cfg(target_family = "unix")]
impl IntoSendBody for UnixStream {
    fn into_body(self) -> SendBody {
        SendBody::from_owned_reader(Box::new(self))
    }
}

impl From<BodyInner> for SendBody {
    fn from(inner: BodyInner) -> Self {
        SendBody {
            inner,
            ended: false,
        }
    }
}

impl<const N: usize> Private for &[u8; N] {}
impl<const N: usize> IntoSendBody for &[u8; N] {
    fn into_body(self) -> SendBody {
        BodyInner::ByteVec(io::Cursor::new(self.as_slice().to_vec())).into()
    }
}

impl Private for () {}
impl IntoSendBody for () {
    fn into_body(self) -> SendBody {
        BodyInner::None.into()
    }
}
