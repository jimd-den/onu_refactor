/// WASM Infrastructure Module
///
/// Exposes the Ọ̀nụ compiler to JavaScript through a `wasm-bindgen` façade.
/// Only compiled when the `wasm` Cargo feature is enabled.
///
/// Build with:
///   cargo build --target wasm32-unknown-unknown --no-default-features --features wasm
#[cfg(feature = "wasm")]
pub mod api;
