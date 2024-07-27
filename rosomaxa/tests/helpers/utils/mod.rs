use crate::utils::{DefaultRandom, Random};
use std::sync::Arc;

pub fn create_test_random() -> Arc<dyn Random> {
    Arc::new(DefaultRandom::default())
}
