///! Contains a mutation operator based on ruin and recreate principle.

use super::*;

/// A mutation operator based on ruin and recreate principle.
pub struct RuinAndRecreate {
    /// A ruin method.
    pub ruin: Box<dyn Ruin + Send + Sync>,
    /// A recreate method.
    pub recreate: Box<dyn Recreate + Send + Sync>,
}

impl RuinAndRecreate {
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

impl Mutation for RuinAndRecreate {
    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let insertion_ctx = self.ruin.run(refinement_ctx, insertion_ctx);

        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}