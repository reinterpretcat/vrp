//! The mutation module specifies building blocks for mutation operator used by evolution.
//!
//! The default implementation of mutation operator is `RuinAndRecreateMutation` which is based on
//! **ruin and recreate** principle, introduced by [`Schrimpf et al. (2000)`].
//!
//! [`Schrimpf et al. (2000)`]: https://www.sciencedirect.com/science/article/pii/S0021999199964136
//!

use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;

mod recreate;
pub use self::recreate::*;

mod ruin;
pub use self::ruin::*;

mod ruin_recreate;
pub use self::ruin_recreate::RuinAndRecreate;

use crate::models::Problem;
use std::sync::Arc;

/// A trait which defines mutation behavior.
pub trait Mutation {
    /// Changes given refinement context and consumes passed insertion context.
    /// Returns an insertion context with potentially new feasible solution.
    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;
}
