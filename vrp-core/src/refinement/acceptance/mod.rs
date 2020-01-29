//! Contains logic which responsible for decision whether some solution is improvement.

use crate::refinement::{Individuum, RefinementContext};

/// Specifies solution acceptance logic.
pub trait Acceptance {
    /// Returns true if solution is considered as improvement.
    fn is_accepted(&self, refinement_ctx: &mut RefinementContext, solution: &Individuum) -> bool;
}

mod greedy;
pub use self::greedy::Greedy;

mod random;
pub use self::random::RandomProbability;
