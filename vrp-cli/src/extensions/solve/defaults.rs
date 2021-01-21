//! Specifies solver default parameters.

use std::sync::Arc;
use vrp_core::utils::{DefaultRandom, Environment, Parallelism, ParallelismDegree};

/// Gets default environment.
pub fn get_default_environment() -> Environment {
    let mut parallelism = Parallelism::new(ParallelismDegree::Full, ParallelismDegree::Full, ParallelismDegree::Full);

    // TODO investigate better defaults
    let (outer, inner) = match parallelism.available_cpus {
        1..=2 => (2, 2),
        3..=8 => (4, 4),
        9..=12 => (6, 4),
        _ => (12, 8),
    };

    parallelism.outer_degree = ParallelismDegree::Limited { max: outer };
    parallelism.inner_degree = ParallelismDegree::Limited { max: inner };

    Environment::new(Arc::new(DefaultRandom::default()), parallelism)
}

/// Gets default population selection size.
pub fn get_default_selection_size(environment: &Environment) -> usize {
    match environment.parallelism.outer_degree {
        ParallelismDegree::Full => environment.parallelism.available_cpus,
        ParallelismDegree::Limited { max } => max,
    }
}
