//! This module reimports a common used types.

// Reimport core types
pub use crate::models::Problem;
pub use crate::models::Solution;
pub use crate::solver::EvolutionConfigBuilder;

// Reimport rosomaxa utils
pub use rosomaxa::utils::compare_floats;
pub use rosomaxa::utils::unwrap_from_result;
pub use rosomaxa::utils::DefaultRandom;
pub use rosomaxa::utils::Environment;
pub use rosomaxa::utils::Random;
