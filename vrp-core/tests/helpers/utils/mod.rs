use rosomaxa::utils::{Environment, Random};
use std::sync::Arc;

pub mod random;

pub fn create_test_environment_with_random(random: Random) -> Arc<Environment> {
    Arc::new(Environment { random, ..Default::default() })
}
