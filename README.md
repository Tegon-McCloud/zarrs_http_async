# zarrs_http_async

A lightweight asynchronous HTTP store for zarrs, designed with WebAssembly (WASM) environments in mind.

zarrs_http_async provides direct asynchronous HTTP access to remote Zarr stores, without requiring a heavyweight storage abstraction or extensive configuration.

The crate aims to make accessing remote Zarr data from WASM applications straightforward, while remaining useful in native asynchronous applications.

## Why zarrs_http_async?

Existing solutions such as OpenDAL and object_store provide comprehensive storage abstractions, but can introduce additional dependencies, feature configuration, and runtime setup. zarrs_http_async takes a more focused approach:
- WASM-friendly: Designed for environments where browser-compatible HTTP access is essential.
- Lightweight: Focuses specifically on HTTP rather than providing a general-purpose storage abstraction.
- Async-native: Performs HTTP requests asynchronously without blocking the execution environment.
- Simple configuration: Minimal setup to access remote Zarr stores.
- zarrs integration: Implements the asynchronous storage interfaces provided by the zarrs ecosystem.
The goal is to make reading remote Zarr datasets from browser-based applications as straightforward as possible.

## Attribution

This crate is a modified derivative of zarrs_http, originally developed by its author(s). It builds upon their original implementation, with modifications and extensions to support asynchronous HTTP access and WASM environments.

Many thanks to the original author(s) for their work on zarrs_http, which provided the foundation for this project.

## Licence
`zarrs_http_async` is licensed under either of
 - the Apache License, Version 2.0 [LICENSE-APACHE](./LICENCE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0> or
 - the MIT license [LICENSE-MIT](./LICENCE-MIT) or <http://opensource.org/licenses/MIT>, at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.