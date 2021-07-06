use crate::construction::heuristics::InsertionContext;
use crate::construction::probing::repair_solution_from_unknown;
use crate::models::Problem;
use crate::solver::mutation::Mutation;
use crate::solver::population::Greedy;
use crate::solver::RefinementContext;
use std::sync::Arc;

/// A mutation operator which performs search in infeasible space.
pub struct InfeasibleSearch {
    inner_mutation: Arc<dyn Mutation + Send + Sync>,
    repeat_count: usize,
}

impl Mutation for InfeasibleSearch {
    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext {
        let new_insertion_ctx = create_relaxed_insertion_ctx(insertion_ctx);
        let mut new_refinement_ctx = create_relaxed_refinement_ctx(refinement_ctx, new_insertion_ctx);

        (0..self.repeat_count).for_each(|_| {
            let new_insertion_ctx = new_refinement_ctx.population.select().next().expect("no individual");
            let new_insertion_ctx = self.inner_mutation.mutate(&new_refinement_ctx, &new_insertion_ctx);
            new_refinement_ctx.population.add(new_insertion_ctx);
        });

        let new_insertion_ctx = new_refinement_ctx.population.select().next().expect("no individual");

        repair_solution_from_unknown(new_insertion_ctx, &|| {
            InsertionContext::new(insertion_ctx.problem.clone(), insertion_ctx.environment.clone())
        })
    }
}

fn create_relaxed_refinement_ctx(
    refinement_ctx: &RefinementContext,
    new_insertion_ctx: InsertionContext,
) -> RefinementContext {
    RefinementContext {
        problem: new_insertion_ctx.problem.clone(),
        population: Box::new(Greedy::new(new_insertion_ctx.problem.clone(), 1, Some(new_insertion_ctx))),
        state: Default::default(),
        quota: refinement_ctx.quota.clone(),
        environment: refinement_ctx.environment.clone(),
        statistics: refinement_ctx.statistics.clone(),
    }
}

fn create_relaxed_insertion_ctx(insertion_ctx: &InsertionContext) -> InsertionContext {
    let problem = insertion_ctx.problem.as_ref();
    let mut insertion_ctx = insertion_ctx.deep_copy();

    insertion_ctx.problem = Arc::new(Problem {
        fleet: problem.fleet.clone(),
        jobs: problem.jobs.clone(),
        locks: problem.locks.clone(),
        // TODO modify constraint
        constraint: problem.constraint.clone(),
        activity: problem.activity.clone(),
        transport: problem.transport.clone(),
        // TODO modify objective
        objective: problem.objective.clone(),
        extras: problem.extras.clone(),
    });

    insertion_ctx
}
