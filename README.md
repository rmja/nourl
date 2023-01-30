# A simple Url primitive
[![crates.io](https://img.shields.io/crates/v/nourl.svg)](https://crates.io/crates/nourl)

This crate provides a simple `Url` type that can be used in embedded `no_std` environments.

If you are missing a feature or would like to add a new scheme, please raise an issue or a PR.

The crate runs on stable rust.

## Example
```rust
let url = Url::parse("http://localhost/foo/bar").unwrap();
assert_eq!(url.scheme(), UrlScheme::HTTP);
assert_eq!(url.host(), "localhost");
assert_eq!(url.port_or_default(), 80);
assert_eq!(url.path(), "/foo/bar");
```

The implementation is heavily inspired (close to copy/pase) from the Url type in [reqwless](https://github.com/drogue-iot/reqwless).