#[cfg(not(target_arch = "wasm32"))]
pub mod generate;

pub mod import;
pub mod solve;
