use crate::construction::states::InsertionContext;
use crate::refinement::RefinementContext;

mod recreate;
pub use self::recreate::*;

mod ruin;
pub use self::ruin::*;

/// Mutates given insertion context.
pub trait Mutator {
    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;
}

/// A mutator which implements ruin and recreate metaheuristic.
pub struct RuinAndRecreateMutator {
    pub recreate: Box<dyn Recreate>,
    pub ruin: Box<dyn Ruin>,
}

impl Default for RuinAndRecreateMutator {
    fn default() -> Self {
        Self { recreate: Box::new(CompositeRecreate::default()), ruin: Box::new(CompositeRuin::default()) }
    }
}

impl RuinAndRecreateMutator {
    /// Creates a new instance of [`RuinAndRecreateMutator`].
    pub fn new(recreate: Box<dyn Recreate>, ruin: Box<dyn Ruin>) -> Self {
        Self { recreate, ruin }
    }
}

impl Mutator for RuinAndRecreateMutator {
    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let insertion_ctx = self.ruin.run(&refinement_ctx, insertion_ctx);
        let insertion_ctx = self.recreate.run(&refinement_ctx, insertion_ctx);

        insertion_ctx
    }
}
