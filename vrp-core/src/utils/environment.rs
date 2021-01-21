//! Contains environment specific logic.

use crate::utils::Random;
use std::sync::Arc;

/// Keeps track of environment specific information which influences algorithm behavior.
#[derive(Clone)]
pub struct Environment {
    /// A wrapper on random generator.
    pub random: Arc<dyn Random + Send + Sync>,

    /// Keeps data parallelism settings.
    pub parallelism: Parallelism,
}

impl Environment {
    /// Creates an instance of `Environment`.
    pub fn new(random: Arc<dyn Random + Send + Sync>, parallelism: Parallelism) -> Self {
        Self { random, parallelism }
    }
}

/// Specifies data parallelism settings.
#[derive(Clone)]
pub struct Parallelism {
    /// Amount of total available CPUs.
    pub available_cpus: usize,

    /// A suggestion of outer loops parallelism degree without parallelized inner loops.
    pub max_degree: ParallelismDegree,

    /// A suggestion of outer loops parallelism degree which might include parallelized inner loops.
    pub outer_degree: ParallelismDegree,

    /// A suggestion of inner loops parallelism degree.
    pub inner_degree: ParallelismDegree,
}

impl Parallelism {
    /// Creates an instance of `Parallelism`.
    pub fn new(
        max_degree: ParallelismDegree,
        outer_degree: ParallelismDegree,
        inner_degree: ParallelismDegree,
    ) -> Self {
        Self { available_cpus: get_cpus(), max_degree, outer_degree, inner_degree }
    }
}

/// Specifies degree of data parallelism
#[derive(Clone)]
pub enum ParallelismDegree {
    /// No restrictions, use underlying defaults
    Full,

    /// Limited parallelism. Applies desired degree.
    Limited {
        /// Max degree of parallelism.
        max: usize,
    },
}

#[cfg(not(target_arch = "wasm32"))]
fn get_cpus() -> usize {
    num_cpus::get()
}

#[cfg(target_arch = "wasm32")]
fn get_cpus() -> usize {
    1
}
