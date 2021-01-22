//! Contains environment specific logic.

use crate::utils::{DefaultRandom, Random};
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

impl Default for Environment {
    fn default() -> Self {
        let parallelism = Parallelism::new(ParallelismDegree::Full, ParallelismDegree::Full, ParallelismDegree::Full);

        // TODO investigate better defaults
        /*let (outer, inner) = match parallelism.available_cpus {
            1..=2 => (2, 2),
            3..=8 => (4, 4),
            9..=12 => (6, 4),
            _ => (12, 8),
        };

        parallelism.outer_degree = ParallelismDegree::Limited { max: outer };
        parallelism.inner_degree = ParallelismDegree::Limited { max: inner };*/

        Environment::new(Arc::new(DefaultRandom::default()), parallelism)
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
