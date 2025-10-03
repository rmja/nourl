# A simple Url primitive
[![CI](https://github.com/rmja/nourl/actions/workflows/ci.yml/badge.svg)](https://github.com/rmja/nourl/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/nourl.svg)](https://crates.io/crates/nourl)

This crate provides a simple `Url` type that can be used in embedded `no_std` environments.

If you are missing a feature or would like to add a new scheme, please raise an issue or a PR.

The crate runs on stable rust.

## Example
```rust
let url = Url::parse("http://localhost/foo/bar?foo=bar&hello=world").unwrap();
assert_eq!(url.scheme(), UrlScheme::HTTP);
assert_eq!(url.host(), "localhost");
assert_eq!(url.port_or_default(), 80);
assert_eq!(url.path(), "/foo/bar");
assert_eq!(url.query(), Some("foo=bar&hello=world"));
```

The implementation is heavily inspired (close to copy/paste) from the Url type in [reqwless](https://github.com/drogue-iot/reqwless).
