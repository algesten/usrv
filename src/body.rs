use std::io::{self, Cursor, Read};

/// Default max body size for read_to_string() and read_to_vec().
const MAX_BODY_SIZE: u64 = 10 * 1024 * 1024;

/// A request body.
///
/// Similar to `ureq`'s Body, this provides multiple ways to read the request body
/// and a builder to configure how it is read.
pub struct Body {
    source: BodySource,
    info: BodyInfo,
}

enum BodySource {
    Reader(Box<dyn Read + Send + Sync>),
}

#[derive(Default, Clone)]
struct BodyInfo {
    mime_type: Option<String>,
    charset: Option<String>,
    content_length: Option<u64>,
}

impl Body {
    /// Create an empty body.
    pub fn empty() -> Body {
        BodyBuilder::new().data(Vec::<u8>::new())
    }

    /// Builder for creating a body.
    ///
    /// This is useful for tests. In a usrv situation, the `Body` will be
    /// read from the incoming request.
    pub fn builder() -> BodyBuilder {
        BodyBuilder::new()
    }

    /// The mime-type part of the `Content-Type` header, if known.
    pub fn mime_type(&self) -> Option<&str> {
        self.info.mime_type.as_deref()
    }

    /// The charset part of the `Content-Type` header, if known.
    pub fn charset(&self) -> Option<&str> {
        self.info.charset.as_deref()
    }

    /// The content length, if known.
    pub fn content_length(&self) -> Option<u64> {
        self.info.content_length
    }

    /// Borrow this body as a reader with default configuration.
    ///
    /// Note: reader is unlimited by default. Use `with_config()` to set a limit.
    pub fn as_reader(&mut self) -> BodyReader {
        self.with_config().reader()
    }

    /// Consume this body into an owned reader with default configuration.
    pub fn into_reader(self) -> BodyReader<'static> {
        self.into_with_config().reader()
    }

    /// Read the body into a `String` using default configuration:
    /// - limited to 10MB
    /// - replaces invalid UTF-8 with `?`
    pub fn read_to_string(&mut self) -> Result<String, crate::Error> {
        self.with_config()
            .limit(MAX_BODY_SIZE)
            .lossy_utf8(true)
            .read_to_string()
    }

    /// Read the body into a `Vec<u8>` using default configuration:
    /// - limited to 10MB
    pub fn read_to_vec(&mut self) -> Result<Vec<u8>, crate::Error> {
        self.with_config().limit(MAX_BODY_SIZE).read_to_vec()
    }

    /// Read the body with configuration while borrowing `self`.
    pub fn with_config(&mut self) -> BodyWithConfig<'_> {
        BodyWithConfig::new(BodySourceRef::Shared(&mut self.source))
    }

    /// Read the body with configuration, consuming `self`.
    pub fn into_with_config(self) -> BodyWithConfig<'static> {
        BodyWithConfig::new(BodySourceRef::Owned(self.source))
    }
}

/// Builder for creating a request body.
pub struct BodyBuilder {
    info: BodyInfo,
    limit: Option<u64>,
}

impl BodyBuilder {
    fn new() -> Self {
        BodyBuilder {
            info: BodyInfo::default(),
            limit: None,
        }
    }

    /// Set the mime type (affects only metadata; does not set headers).
    pub fn mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.info.mime_type = Some(mime_type.into());
        self
    }

    /// Set the charset (affects only metadata; does not set headers).
    pub fn charset(mut self, charset: impl Into<String>) -> Self {
        self.info.charset = Some(charset.into());
        self
    }

    /// Set a length limit for the created body when built from in-memory data.
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Create a body from in-memory data.
    pub fn data(mut self, data: impl Into<Vec<u8>>) -> Body {
        let data: Vec<u8> = data.into();
        let len = data.len() as u64;
        let limit = self.limit.unwrap_or(len);
        self.info.content_length = Some(limit.min(len));
        self.reader(Cursor::new(data))
    }

    /// Create a body from a streaming reader.
    pub fn reader(self, reader: impl Read + Send + Sync + 'static) -> Body {
        Body {
            source: BodySource::Reader(Box::new(reader)),
            info: self.info,
        }
    }
}

enum BodySourceRef<'a> {
    Shared(&'a mut BodySource),
    Owned(BodySource),
}

impl<'a> Read for BodySourceRef<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            BodySourceRef::Shared(BodySource::Reader(r)) => r.read(buf),
            BodySourceRef::Owned(ref mut src) => match src {
                BodySource::Reader(r) => r.read(buf),
            },
        }
    }
}

/// Configuration of how to read the body.
pub struct BodyWithConfig<'a> {
    source: BodySourceRef<'a>,
    limit: u64,
    lossy_utf8: bool,
}

impl<'a> BodyWithConfig<'a> {
    fn new(source: BodySourceRef<'a>) -> Self {
        BodyWithConfig {
            source,
            limit: u64::MAX,
            lossy_utf8: false,
        }
    }

    /// Limit how many bytes to read before erroring.
    pub fn limit(mut self, value: u64) -> Self {
        self.limit = value;
        self
    }

    /// Replace invalid utf-8 with `?` when reading to string.
    pub fn lossy_utf8(mut self, value: bool) -> Self {
        self.lossy_utf8 = value;
        self
    }

    fn build_reader(self) -> LimitReader<BodySourceRef<'a>> {
        LimitReader::new(self.source, self.limit)
    }

    /// Create a `Read`er for streaming the body.
    pub fn reader(self) -> BodyReader<'a> {
        BodyReader {
            inner: self.build_reader(),
        }
    }

    /// Read the body to a `String` using current configuration.
    pub fn read_to_string(self) -> Result<String, crate::Error> {
        let lossy = self.lossy_utf8;
        let mut reader = self.build_reader();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).map_err(crate::Error::Io)?;
        if lossy {
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        } else {
            String::from_utf8(bytes).map_err(crate::Error::FromUtf8)
        }
    }

    /// Read the body to a `Vec<u8>` using current configuration.
    pub fn read_to_vec(self) -> Result<Vec<u8>, crate::Error> {
        let mut reader = self.build_reader();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).map_err(crate::Error::Io)?;
        Ok(bytes)
    }
}

/// Reader wrapper that enforces a byte limit.
pub struct BodyReader<'a> {
    inner: LimitReader<BodySourceRef<'a>>,
}

impl<'a> Read for BodyReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

struct LimitReader<R> {
    reader: R,
    limit: u64,
    left: u64,
}

impl<R> LimitReader<R> {
    fn new(reader: R, limit: u64) -> Self {
        LimitReader {
            reader,
            limit,
            left: limit,
        }
    }
}

impl<R: Read> Read for LimitReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.left == 0 {
            return Err(crate::Error::BodyExceedsLimit(self.limit).into());
        }

        let max = (self.left.min(usize::MAX as u64) as usize).min(buf.len());
        let n = self.reader.read(&mut buf[..max])?;
        self.left -= n as u64;
        Ok(n)
    }
}
