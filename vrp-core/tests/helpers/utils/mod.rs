use crate::utils::{DefaultRandom, Environment, Parallelism, ParallelismDegree, Random};
use std::sync::Arc;

pub mod random;

pub fn create_test_environment() -> Arc<Environment> {
    create_test_environment_with_random(Arc::new(DefaultRandom::default()))
}

pub fn create_test_environment_with_random(random: Arc<dyn Random + Send + Sync>) -> Arc<Environment> {
    Arc::new(Environment {
        random,
        parallelism: Parallelism::new(
            ParallelismDegree::Full,
            ParallelismDegree::Limited { max: 4 },
            ParallelismDegree::Limited { max: 4 },
        ),
    })
}
