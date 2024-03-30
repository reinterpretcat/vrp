use rosomaxa::prelude::{DefaultEnvironment, DefaultRandom};

pub mod random;

pub fn create_test_environment_with_random(random: DefaultRandom) -> DefaultEnvironment {
    DefaultEnvironment { random, ..Default::default() }
}
