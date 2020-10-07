//! The mutation module specifies building blocks for mutation operator used by evolution.
//!
//! The default implementation of mutation operator is `RuinAndRecreateMutation` which is based on
//! **ruin and recreate** principle, introduced by [`Schrimpf et al. (2000)`].
//!
//! [`Schrimpf et al. (2000)`]: https://www.sciencedirect.com/science/article/pii/S0021999199964136
//!

use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;

mod local;
pub use self::local::*;

mod recreate;
pub use self::recreate::*;

mod ruin;
pub use self::ruin::*;

mod naive_branching;
pub use self::naive_branching::NaiveBranching;

mod ruin_recreate;
pub use self::ruin_recreate::RuinAndRecreate;

mod weighted_composite;
pub use self::weighted_composite::WeightedComposite;

use crate::models::Problem;
use std::sync::Arc;

/// A trait which defines mutation behavior.
pub trait Mutation {
    /// Mutates passed insertion context.
    fn mutate_one(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;

    /// Mutates passed insertion contexts.
    fn mutate_all(
        &self,
        refinement_ctx: &RefinementContext,
        individuals: Vec<InsertionContext>,
    ) -> Vec<InsertionContext>;
}
