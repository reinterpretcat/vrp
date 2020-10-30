///! Contains a mutation operator based on ruin and recreate principle.
use super::*;
use crate::algorithms::nsga2::Objective;
use crate::utils::parallel_into_collect;
use std::cmp::Ordering;

/// A mutation operator based on ruin and recreate principle.
pub struct RuinAndRecreate {
    ruin: Box<dyn Ruin + Send + Sync>,
    recreate: Box<dyn Recreate + Send + Sync>,
    pre_local_search: (Box<dyn LocalSearch + Send + Sync>, f64),
    post_local_search: (Box<dyn LocalSearch + Send + Sync>, f64),
}

impl RuinAndRecreate {
    /// Creates a new instance of `RuinAndRecreate` using given ruin and recreate methods.
    pub fn new(
        recreate: Box<dyn Recreate + Send + Sync>,
        ruin: Box<dyn Ruin + Send + Sync>,
        pre_local_search: (Box<dyn LocalSearch + Send + Sync>, f64),
        post_local_search: (Box<dyn LocalSearch + Send + Sync>, f64),
    ) -> Self {
        Self { recreate, pre_local_search, ruin, post_local_search }
    }

    /// Creates a new instance of `RuinAndRecreate` using default ruin and recreate methods.
    pub fn new_from_problem(
        problem: Arc<Problem>,
        pre_local_search: (Box<dyn LocalSearch + Send + Sync>, f64),
        post_local_search: (Box<dyn LocalSearch + Send + Sync>, f64),
    ) -> Self {
        Self {
            recreate: Box::new(CompositeRecreate::new_from_problem(problem.clone())),
            pre_local_search,
            ruin: Box::new(CompositeRuin::new_from_problem(problem)),
            post_local_search,
        }
    }
}

impl Mutation for RuinAndRecreate {
    fn mutate_one(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let (insertion_ctx, is_improved) =
            maybe_apply_local_search(refinement_ctx, insertion_ctx, &self.pre_local_search);

        if is_improved {
            return insertion_ctx;
        }

        let insertion_ctx = self.ruin.run(refinement_ctx, insertion_ctx);
        let insertion_ctx = self.recreate.run(refinement_ctx, insertion_ctx);

        maybe_apply_local_search(refinement_ctx, insertion_ctx, &self.post_local_search).0
    }

    fn mutate_all(
        &self,
        refinement_ctx: &RefinementContext,
        individuals: Vec<InsertionContext>,
    ) -> Vec<InsertionContext> {
        parallel_into_collect(individuals, |insertion_ctx| self.mutate_one(refinement_ctx, insertion_ctx))
    }
}

fn maybe_apply_local_search(
    refinement_ctx: &RefinementContext,
    insertion_ctx: InsertionContext,
    local_search: &(Box<dyn LocalSearch + Send + Sync>, f64),
) -> (InsertionContext, bool) {
    if insertion_ctx.random.is_hit(local_search.1) {
        if let Some(new_insertion_ctx) = local_search.0.explore(refinement_ctx, &insertion_ctx) {
            let is_improved =
                refinement_ctx.problem.objective.total_order(&insertion_ctx, &new_insertion_ctx) == Ordering::Greater;
            return (new_insertion_ctx, is_improved);
        }
    }

    (insertion_ctx, false)
}
