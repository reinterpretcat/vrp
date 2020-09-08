/// Returns amount of CPUs.
#[cfg(not(target_arch = "wasm32"))]
pub fn get_cpus() -> usize {
    num_cpus::get()
}

/// Returns amount of CPUs.
#[cfg(target_arch = "wasm32")]
pub fn get_cpus() -> usize {
    1
}
