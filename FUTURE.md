Borrowed extractors with GATs (future plan)

Problem
- Today `Query<T>` requires `T: DeserializeOwned` → allocates for captured fields.
- We removed `RefArg`; `Arg` now extracts from `&http::Request<Body>` but still returns owned values.

Goal
- Allow extractors to return values that borrow directly from the incoming request/URI without cloning, while keeping handler ergonomics and consistent semantics.

Design (requires GATs in our API)
- Redefine `Arg` to expose a lifetime-parameterized output that can borrow from the request:
  - type Output<'req>;
  - fn from_request<'req>(&S, &'req http::Request<Body>) -> Result<Output<'req>, Rejection>;
- Update handler plumbing so each parameter `A: Arg<S>` contributes an `A::Output<'req>` where `'req` is the concrete request lifetime.

Planned borrowed extractors
- RefQuery<T> where `T: Deserialize<'req>`; uses `serde_urlencoded`, treats `+` as space, percent-decodes; supports `&'req str`/`Cow<'req, str>`.
- RefPath<T> where `T: Deserialize<'req>`; borrows path captures from the matcher.
- Header views that borrow from the header map when possible.


Extractor roadmap (priority)

P0 — Core
- Form<T, const MAX: u64>: `application/x-www-form-urlencoded` body via `serde_urlencoded`.

P1 — High value
- MatchedPath: expose the registered route pattern that matched the handler.
- Host: effective host (authority header or URI host fallback).
- Body: extract `Body` as the last parameter, consuming the request.
- Extension<T>: extract from `http::Extensions` (T: Clone + Send + Sync + 'static).
- State<S>: extractor form of app state to allow non-leading placement (S: Clone).

P2 — Nice to have (feature-gated where needed)
- TypedHeader<H> (feature `typed-headers`): typed header extraction using the `headers` crate.
- CookieJar (feature `cookies`): parse and set cookies from/to headers.
- OriginalUri: alias of `http::Uri` until rewrite/middleware support exists.
