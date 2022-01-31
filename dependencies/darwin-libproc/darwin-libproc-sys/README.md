# darwin-libproc-sys

[![Latest Version](https://img.shields.io/crates/v/darwin-libproc-sys.svg)](https://crates.io/crates/darwin-libproc-sys)
[![Latest Version](https://docs.rs/darwin-libproc-sys/badge.svg)](https://docs.rs/darwin-libproc-sys)
![Minimum rustc version](https://img.shields.io/badge/rustc-1.36+-green.svg)
![Apache 2.0 OR MIT licensed](https://img.shields.io/badge/license-Apache2.0%2FMIT-blue.svg)
![Platforms supported](https://img.shields.io/badge/platform-macOS-brightgreen)
![Unsafe](https://img.shields.io/badge/unsafe-FFI-red.svg)

> Low-level Rust bindings for `libproc` of macOS

This crate provides unsafe low-level bindings for `libproc`,
based on the [xnu-4570.1.46](https://opensource.apple.com/source/xnu/xnu-4570.1.46/) sources
(used in macOS 10.13).

See [darwin-libproc](https://crates.io/crates/darwin-libproc) crate for idiomatic safe wrappers for these bindings.

## License

Licensed under either of [Apache License 2.0](https://github.com/heim-rs/darwin-libproc/blob/master/LICENSE-APACHE)
or [MIT license](https://github.com/heim-rs/darwin-libproc/blob/master/LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you,
as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
