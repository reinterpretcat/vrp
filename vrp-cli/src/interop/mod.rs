//! Provides bindings to use the solver from other languages: a C ABI, python behind the
//! `py_bindings` feature, and javascript on the `wasm32` target.
//!
//! All of them expose the same operations and are thin adapters over [`json`], which owns the
//! implementation. Only the representation of arguments, results and errors differs.

pub mod json;

#[cfg(not(target_arch = "wasm32"))]
#[allow(unsafe_code)]
mod c_api;

#[cfg(feature = "py_bindings")]
#[cfg(not(target_arch = "wasm32"))]
mod python;

#[cfg(target_arch = "wasm32")]
mod wasm;
