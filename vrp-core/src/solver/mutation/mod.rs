use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;

mod recreate;
pub use self::recreate::*;

mod ruin;
pub use self::ruin::*;

/// Mutates given insertion context.
pub trait Mutation {
    fn mutate(&self, refinement_ctx: &mut RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;
}

/// A mutation which implements ruin and recreate metaheuristic.
pub struct RuinAndRecreateMutation {
    pub recreate: Box<dyn Recreate>,
    pub ruin: Box<dyn Ruin>,
}

impl Default for RuinAndRecreateMutation {
    fn default() -> Self {
        Self { recreate: Box::new(CompositeRecreate::default()), ruin: Box::new(CompositeRuin::default()) }
    }
}

impl RuinAndRecreateMutation {
    /// Creates a new instance of [`RuinAndRecreateMutation`].
    pub fn new(recreate: Box<dyn Recreate>, ruin: Box<dyn Ruin>) -> Self {
        Self { recreate, ruin }
    }
}

impl Mutation for RuinAndRecreateMutation {
    fn mutate(&self, refinement_ctx: &mut RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let insertion_ctx = self.ruin.run(refinement_ctx, insertion_ctx);

        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}
