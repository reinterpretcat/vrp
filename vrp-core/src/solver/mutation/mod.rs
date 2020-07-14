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
use crate::models::Problem;
use std::sync::Arc;

/// A trait which defines mutation behavior.
pub trait Mutation {
    /// Changes given refinement context and consumes passed insertion context.
    /// Returns an insertion context with potentially new feasible solution.
    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;
}

/// A mutation operator based on ruin and recreate principle.
pub struct RuinAndRecreateMutation {
    /// A ruin method.
    pub ruin: Box<dyn Ruin + Send + Sync>,
    /// A recreate method.
    pub recreate: Box<dyn Recreate + Send + Sync>,
}

impl RuinAndRecreateMutation {
    /// Creates a new instance of `RuinAndRecreateMutation` using given ruin and recreate methods.
    pub fn new(recreate: Box<dyn Recreate + Send + Sync>, ruin: Box<dyn Ruin + Send + Sync>) -> Self {
        Self { recreate, ruin }
    }

    /// Creates a new instance of `RuinAndRecreateMutation` using default ruin and recreate methods.
    pub fn new_from_problem(problem: Arc<Problem>) -> Self {
        Self {
            recreate: Box::new(CompositeRecreate::new_from_problem(problem.clone())),
            ruin: Box::new(CompositeRuin::new_from_problem(problem)),
        }
    }
}

impl Mutation for RuinAndRecreateMutation {
    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let insertion_ctx = self.ruin.run(refinement_ctx, insertion_ctx);

        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}
