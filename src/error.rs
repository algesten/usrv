use std::fmt;
use std::io;

/// usrv errors.
#[derive(Debug)]
pub enum Error {
    /// IO error wrapper.
    Io(io::Error),
    /// Body exceeded configured size limit.
    BodyExceedsLimit(u64),
    /// Some error with serde handling.
    #[cfg(feature = "json")]
    SerdeJson(serde_json::Error),
    /// UTF-8 decoding error when constructing a `String`.
    FromUtf8(std::string::FromUtf8Error),
}

#[cfg(feature = "json")]
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::SerdeJson(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {}", e),
            Error::BodyExceedsLimit(l) => write!(f, "body exceeds limit: {} bytes", l),
            #[cfg(feature = "json")]
            Error::SerdeJson(e) => write!(f, "serde json: {}", e),
            Error::FromUtf8(e) => write!(f, "utf8: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::BodyExceedsLimit(_) => None,
            #[cfg(feature = "json")]
            Error::SerdeJson(e) => Some(e),
            Error::FromUtf8(e) => Some(e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<Error> for io::Error {
    fn from(error: Error) -> Self {
        match error {
            Error::Io(e) => e,
            other => io::Error::other(other),
        }
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(error: std::string::FromUtf8Error) -> Self {
        Error::FromUtf8(error)
    }
}
