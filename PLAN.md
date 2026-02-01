# usrv 0.0.1 Release Plan

## Context

usrv is a minimal synchronous HTTP/1.1 server framework at v0.0.0. The routing/extraction/response
layer works and has 34 passing unit tests, but there are bugs from the code review, no integration
tests exercising extractor combinations, no Form/MatchedPath extractors, and no actual TCP server.
This plan gets to a shippable 0.0.1.

## Scope

1. Bug fixes (LimitReader, silent route conflicts, 405 Method Not Allowed)
2. New extractors: `Form<T>` (P0) and `MatchedPath` (P1)
3. Comprehensive integration test suite
4. Basic single-threaded TCP server using ureq-proto
5. Minor cleanup (unused dep, typo)

---

## Phase 1: Bug Fixes

### 1A. LimitReader off-by-one (`src/body.rs:248-259`)

**Bug:** When a body is exactly `limit` bytes, `left` reaches 0, and the next `read()` returns
`Err(BodyExceedsLimit)` instead of `Ok(0)`. This means `read_to_end` always errors when the body
is exactly at the limit.

**Fix:** When `left == 0`, probe the underlying reader for one more byte. If it returns 0 (true
EOF), return `Ok(0)`. If it has more data, return the error. The probed byte is discarded but
that's fine since we're erroring anyway.

```rust
fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    if self.left == 0 {
        let mut probe = [0u8; 1];
        return match self.reader.read(&mut probe)? {
            0 => Ok(0),
            _ => Err(crate::Error::BodyExceedsLimit(self.limit).into()),
        };
    }
    let max = (self.left.min(usize::MAX as u64) as usize).min(buf.len());
    let n = self.reader.read(&mut buf[..max])?;
    self.left -= n as u64;
    Ok(n)
}
```

### 1B. Silent route conflicts (`src/router.rs:93-98`)

**Bug:** `trie.insert()` errors are silently ignored with `continue`.

**Fix:** Panic with a descriptive message. Route conflicts are programmer errors caught at startup
(same convention as axum/actix-web).

```rust
if let Err(e) = trie.insert(p.path.as_ref(), p.id) {
    panic!("Route conflict for '{}': {}", p.path, e);
}
```

### 1C. 405 Method Not Allowed (`src/router.rs`, `src/service.rs`)

**Bug:** Path match + method mismatch returns 404 instead of 405.

**Fix:**
- Add `MethodNotAllowed(String)` variant to `CallResult` (the String is the `Allow` header value)
- In `BuiltRouter::call`, when path matches but method doesn't, collect allowed methods and return
  `CallResult::MethodNotAllowed(allowed)`
- In `Service::call`, build a 405 response with `Allow` header

### 1D. Minor cleanup

- Rename `prepare_extracters` -> `prepare_extractors` in `src/extract/mod.rs` (typo, pub(crate))
- Remove unused `percent-encoding` dependency from `Cargo.toml`
- Remove commented-out `body_mode()` code in `src/send_body.rs`

---

## Phase 2: New Extractors

### 2A. Form<T> extractor

**New file:** `src/extract/form.rs`
**Modify:** `src/extract/mod.rs`, `Cargo.toml`

Mirrors the `Json<T>` pattern:
- `pub struct Form<T, const MAX: u64 = 10_485_760>(pub T)`
- Implements `FromRequest<S>` only (consumes body)
- Validates `Content-Type: application/x-www-form-urlencoded` (lenient when missing, like Json)
- Reads body with limit, deserializes via `serde_urlencoded::from_bytes`
- `FormRejection` returns 400

**Feature gating:** New `form` feature in Cargo.toml:
```toml
form = ["dep:serde_urlencoded", "dep:form_urlencoded"]
default = ["query", "json", "form"]
```

### 2B. MatchedPath extractor

**New file:** `src/extract/matched_path.rs`
**Modify:** `src/extract/mod.rs`, `src/router.rs`

- `pub struct MatchedPath(pub(crate) Arc<str>)` with `pub fn as_str(&self) -> &str`
- Implements `FromRequestParts<S>` (reads from extensions)
- `MatchedPathRejection` returns 500 (missing = server config error)
- In `BuiltRouter::call`, insert `MatchedPath(entry.path.clone())` into request extensions
  alongside `PathParams`

---

## Phase 3: Comprehensive Test Suite

**New file:** `tests/integration.rs`

Shared helpers: `read_body_string`, `get`, `post_json`, `post_form`

### Test Matrix

**A. Handler signatures (basics)**
1. `fn()` - no args, no return
2. `fn() -> &'static str` - static string return
3. `fn() -> String` - owned return
4. `fn(Request<Body>) -> String` - full request
5. `fn(&mut State) -> String` - mutable state
6. `fn(&mut State, Request<Body>) -> String` - state + request

**B. Individual extractors as leading param + body**
7. `fn(Method, Request<Body>) -> String`
8. `fn(Uri, Request<Body>) -> String`
9. `fn(Version, Request<Body>) -> &str`
10. `fn(HeaderMap, Request<Body>) -> String`
11. `fn(Path<T>, Request<Body>) -> String` (named struct)
12. `fn(Path<(String,)>, Request<Body>) -> String` (tuple)
13. `fn(Query<T>, Request<Body>) -> String`
14. `fn(Host, Request<Body>) -> String`
15. `fn(Extension<T>, Request<Body>) -> String`
16. `fn(MatchedPath, Request<Body>) -> String`

**C. Body extractors as final param**
17. `fn(Method, Json<T>) -> String`
18. `fn(Method, Form<T>) -> String`
19. `fn(Method, Body) -> String`
20. `fn(Method, Bytes) -> String`
21. `fn(Method, Text) -> String`
22. `fn(Query<T>) -> String` (Query as sole/final param via FromRequest)

**D. Multi-extractor combinations (the borrowing stress tests)**
23. `fn(Method, Uri, Version, Request<Body>) -> String` - 3 parts + request
24. `fn(Path<T>, Query<T>, Json<P>) -> String` - path + query + json
25. `fn(Method, Path<T>, Host, Text) -> String` - 3 parts + text
26. `fn(MatchedPath, Path<T>, Query<Q>, Bytes) -> String` - 4 extractors
27. `fn(&mut State, Method, Path<T>, Json<P>) -> String` - state + multi-extractor
28. `fn(&mut State, MatchedPath, Query<Q>, Text) -> String` - state + matched + query + text

**E. Option/Result wrappers**
29. `Option<Host>` present / absent
30. `Option<Json<T>>` as final param - invalid json becomes None
31. `Result<Host, HostRejection>` Ok / Err
32. `Result<Json<T>, JsonRejection>` as final param

**F. Router behavior**
33. GET returns 200, POST on GET-only path returns 405 with `Allow` header
34. Unknown path returns 404
35. Multiple methods on same path (GET + POST) route correctly
36. Nested path params `/a/{x}/b/{y}`
37. Closure handler capturing `Arc<Mutex<T>>`

**G. LimitReader fix validation**
38. Body exactly at limit succeeds
39. Body one byte over limit fails
40. Bytes<N> with exact limit
41. Text<N> with exact limit
42. Json<T, N> with exact limit

**H. Form<T> tests**
43. Correct content-type parses OK
44. Missing content-type parses OK
45. Wrong content-type returns 400
46. Invalid body returns 400
47. Size limit exceeded returns 400
48. Percent-encoded and plus-as-space

**I. MatchedPath tests**
49. Returns pattern string `/users/{id}`, not actual `/users/42`
50. Combined with Path<T> in same handler

---

## Phase 4: Transport Layer + Server (modeled on ureq's pattern)

Following ureq's sans-IO bridging design: the ureq-proto state machine (Reply<State>) operates
on raw buffers, and a **Transport** abstraction owns those buffers and bridges to real I/O. An
**Acceptor** produces Transport instances for incoming connections. The server is generic over the
acceptor, allowing bring-your-own-transport.

### 4A. Transport traits (`src/transport.rs` — **new**)

#### Buffers trait (ported from ureq)

Same interface as `ureq::unversioned::transport::Buffers`. Manages input/output buffers that
the sans-IO state machine reads from and writes to.

```rust
pub trait Buffers {
    /// Output buffer for writing data (request headers, body encoding).
    fn output(&mut self) -> &mut [u8];
    /// Unconsumed input bytes (response data from network).
    fn input(&self) -> &[u8];
    /// Space to append new input bytes from the network.
    fn input_append_buf(&mut self) -> &mut [u8];
    /// Mark `amount` bytes as appended to input.
    fn input_appended(&mut self, amount: usize);
    /// Mark `amount` bytes as consumed from input.
    fn input_consume(&mut self, amount: usize);
    /// Scratch space + output buffer (for body encoding).
    fn tmp_and_output(&mut self) -> (&mut [u8], &mut [u8]);
    /// Whether unconsumed input is available and making progress.
    fn can_use_input(&self) -> bool;
}
```

#### LazyBuffers (ported from ureq)

Default implementation using a `ConsumeBuf` (Vec with filled/consumed cursors, auto-shift).
Port both `ConsumeBuf` and `LazyBuffers` from ureq into usrv.

```rust
pub struct LazyBuffers { ... }  // same as ureq's
impl LazyBuffers {
    pub fn new(input_size: usize, output_size: usize) -> Self;
}
impl Buffers for LazyBuffers { ... }
```

#### Transport trait

Adapted from ureq's `Transport` — simplified for server (no per-operation timeout for 0.0.1):

```rust
pub trait Transport: Debug + Send + Sync + 'static {
    /// Provide buffers for this transport.
    fn buffers(&mut self) -> &mut dyn Buffers;
    /// Transmit `amount` bytes from the output buffer to the peer.
    fn transmit_output(&mut self, amount: usize) -> Result<(), io::Error>;
    /// Wait for input, fill the input buffer. Returns true if progress was made.
    fn await_input(&mut self) -> Result<bool, io::Error>;
    /// Check if the transport is still open.
    fn is_open(&mut self) -> bool;
    /// Whether this transport is TLS-wrapped.
    fn is_tls(&self) -> bool { false }
}
```

#### Acceptor trait

Server-side analog of ureq's `Connector`. Produces Transport instances for incoming connections.

```rust
pub trait Acceptor: Debug + Send + Sync + 'static {
    type Transport: Transport;
    /// Block until a new connection arrives, returning a transport.
    fn accept(&self) -> Result<Self::Transport, io::Error>;
    /// The local address being listened on.
    fn local_addr(&self) -> Result<SocketAddr, io::Error>;
}
```

### 4B. Default TCP implementation (`src/transport/tcp.rs` — **new**)

```rust
pub struct TcpAcceptor {
    listener: TcpListener,
}

impl TcpAcceptor {
    pub fn bind(addr: impl ToSocketAddrs) -> io::Result<Self>;
}

impl Acceptor for TcpAcceptor {
    type Transport = TcpTransport;
    fn accept(&self) -> io::Result<TcpTransport>;
    fn local_addr(&self) -> io::Result<SocketAddr>;
}

pub struct TcpTransport {
    stream: TcpStream,
    buffers: LazyBuffers,
}

impl Transport for TcpTransport {
    fn buffers(&mut self) -> &mut dyn Buffers { &mut self.buffers }
    fn transmit_output(&mut self, amount: usize) -> io::Result<()> {
        // self.stream.write_all(&self.buffers.output()[..amount])
    }
    fn await_input(&mut self) -> io::Result<bool> {
        // read into self.buffers.input_append_buf(), call input_appended()
    }
    fn is_open(&mut self) -> bool { /* probe stream */ }
}
```

### 4C. Server (`src/server.rs` — **new**)

Generic over Acceptor:

```rust
pub struct Server<A: Acceptor = TcpAcceptor> {
    acceptor: A,
}

impl Server<TcpAcceptor> {
    /// Bind a TCP server.
    pub fn bind(addr: impl ToSocketAddrs) -> io::Result<Self>;
}

impl<A: Acceptor> Server<A> {
    /// Create from a custom acceptor (bring-your-own-transport).
    pub fn new(acceptor: A) -> Self;

    pub fn local_addr(&self) -> io::Result<SocketAddr>;

    /// Blocking accept loop. S: Clone — use () for stateless, Arc<Mutex<T>> for shared state.
    pub fn run<S: Clone>(&self, service: &Service<S>, state: S) -> io::Result<()>;

    /// Accept and handle one connection (for testing).
    pub fn accept_one<S: Clone>(&self, service: &Service<S>, state: S) -> io::Result<()>;
}
```

### 4D. Connection handler (drives ureq-proto via Transport)

The core loop per connection — all I/O goes through `Transport::transmit_output` and
`Transport::await_input`, with the ureq-proto Reply state machine reading/writing from
`Transport::buffers()`:

```
Per connection (loop for keep-alive):
  1. RecvRequest:
     - transport.await_input() to fill input buffer
     - reply.try_request(transport.buffers().input()) to parse headers
     - transport.buffers().input_consume(consumed)
     - Repeat until headers complete
  2. Proceed -> Send100 | RecvBody | ProvideResponse
  3. If Send100:
     - reply.accept(transport.buffers().output()) writes 100-continue
     - transport.transmit_output(n) sends it
  4. If RecvBody:
     - Loop: transport.await_input(), reply.read(input, body_buf),
       transport.buffers().input_consume()
     - Accumulate body into Vec<u8>
  5. Build usrv Request<Body>:
     - From http::Request<()> headers + body bytes
     - Parse Content-Type for mime_type/charset metadata
  6. service.call(state.clone(), request) -> Response<SendBody>
  7. If body size known, set content-length header on response
  8. reply.provide(Response<()>) -> SendResponse
  9. SendResponse:
     - reply.write(transport.buffers().output()) writes headers
     - transport.transmit_output(n) sends them
  10. SendBody:
      - Read from usrv SendBody into tmp buffer
      - reply.write(tmp, transport.buffers().output()) encodes body
      - transport.transmit_output(n) sends chunks
      - Signal end with reply.write(&[], output)
  11. Cleanup: must_close_connection() -> break or continue loop
```

### 4E. SendBody content_length (`src/send_body.rs`)

Add `pub(crate) fn content_length(&self) -> Option<u64>`:
- `BodyInner::None` -> `Some(0)`
- `BodyInner::ByteVec(v)` -> `Some(v.get_ref().len() as u64)`
- `BodyInner::OwnedReader(_)` -> `None`

Used in step 7 above to set `content-length` and avoid chunked encoding for known-size responses.

### 4F. Server integration tests (`tests/server.rs` — **new**)

- Spawn server on `127.0.0.1:0` in a background thread
- Connect with raw TCP, send HTTP/1.1 request bytes, verify response
- Test keep-alive (multiple requests on one connection)
- Test POST with body
- Test 404 for unknown path

### 4G. Module structure

```
src/transport/
  mod.rs      — Buffers trait, LazyBuffers, Transport trait, Acceptor trait, ConsumeBuf
  tcp.rs      — TcpAcceptor, TcpTransport
src/server.rs — Server struct, connection handler loop
```

**State model:** `S: Clone`, passed to `run`. `()` is Clone. `Arc<T>` is Clone. The `&mut AppState`
pattern is for direct `Service::call` in tests — real servers use `Arc<Mutex<T>>`.

**Future extension:** Acceptor chaining (similar to ureq's Connector chain) for composing TCP + TLS.
Not needed for 0.0.1 but the trait design is compatible with it.

---

## Phase 5: Finalize

- Bump version in `Cargo.toml`: `"0.0.0"` -> `"0.0.1"`
- Keep `ureq-proto` as path dependency for now (not published yet)
- Run `cargo clippy` and `cargo test` to verify everything

---

## File Change Summary

| File | Action | Phase |
|------|--------|-------|
| `src/body.rs` | Fix LimitReader | 1A |
| `src/router.rs` | Panic on conflicts + 405 + insert MatchedPath | 1B, 1C, 2B |
| `src/service.rs` | Handle MethodNotAllowed variant | 1C |
| `src/extract/mod.rs` | Fix typo, add form + matched_path modules | 1D, 2A, 2B |
| `src/extract/form.rs` | **New** - Form<T> extractor | 2A |
| `src/extract/matched_path.rs` | **New** - MatchedPath extractor | 2B |
| `src/send_body.rs` | Remove commented code, add content_length() | 1D, 4E |
| `src/transport/mod.rs` | **New** - Buffers, LazyBuffers, Transport, Acceptor traits | 4A |
| `src/transport/tcp.rs` | **New** - TcpAcceptor, TcpTransport | 4B |
| `src/server.rs` | **New** - Server struct + connection handler | 4C, 4D |
| `src/lib.rs` | Export transport module, Server | 4 |
| `Cargo.toml` | form feature, remove percent-encoding, version bump | 2A, 1D, 5 |
| `tests/integration.rs` | **New** - ~50 integration tests | 3 |
| `tests/server.rs` | **New** - TCP server integration tests | 4F |

## Verification

1. `cargo test` - all existing 34 tests + ~50 new integration + server tests pass
2. `cargo clippy` - no warnings
3. Manual smoke test: run a simple server, curl it

---

## Future: Borrowed Extractors with GATs

### Problem

- Today `Query<T>` requires `T: DeserializeOwned` — allocates for captured fields.
- `FromRequestParts` / `FromRequest` return owned values; there's no way to borrow from the
  incoming request without cloning.

### Goal

Allow extractors to return values that borrow directly from the incoming request/URI without
cloning, while keeping handler ergonomics and consistent semantics.

### Design (requires GATs in our API)

- Redefine `Arg` to expose a lifetime-parameterized output that can borrow from the request:
  - `type Output<'req>;`
  - `fn from_request<'req>(&S, &'req http::Request<Body>) -> Result<Output<'req>, Rejection>;`
- Update handler plumbing so each parameter `A: Arg<S>` contributes an `A::Output<'req>` where
  `'req` is the concrete request lifetime.

### Planned borrowed extractors

- `RefQuery<T>` where `T: Deserialize<'req>`; uses `serde_urlencoded`, treats `+` as space,
  percent-decodes; supports `&'req str` / `Cow<'req, str>`.
- `RefPath<T>` where `T: Deserialize<'req>`; borrows path captures from the matcher.
- Header views that borrow from the header map when possible.

### Extractor roadmap (post-0.0.1)

**P2 — Nice to have (feature-gated where needed)**
- `TypedHeader<H>` (feature `typed-headers`): typed header extraction using the `headers` crate.
- `CookieJar` (feature `cookies`): parse and set cookies from/to headers.
- `OriginalUri`: alias of `http::Uri` until rewrite/middleware support exists.
