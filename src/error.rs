use std::fmt;

/// usrv errors.
#[derive(Debug)]
pub enum Error {
    /// Some error with serde handling.
    #[cfg(feature = "json")]
    SerdeJson(serde_json::Error),
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
            #[cfg(feature = "json")]
            Error::SerdeJson(e) => write!(f, "serde json: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(feature = "json")]
            Error::SerdeJson(e) => Some(e),
        }
    }
}
