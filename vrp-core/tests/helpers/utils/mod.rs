use rosomaxa::utils::{Environment, Random};
use std::sync::Arc;

pub mod random;

pub fn create_test_environment_with_random(random: Arc<dyn Random + Send + Sync>) -> Arc<Environment> {
    let mut environment = Environment::default();
    environment.random = random;

    Arc::new(environment)
}
