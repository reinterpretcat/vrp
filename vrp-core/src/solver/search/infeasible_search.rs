use crate::construction::heuristics::*;
use crate::construction::probing::repair_solution_from_unknown;
use crate::models::problem::Job;
use crate::models::*;
use crate::solver::*;
use rosomaxa::population::Alternative;
use std::sync::Arc;

/// A mutation operator which performs search in infeasible space.
pub struct InfeasibleSearch {
    inner_search: TargetSearchOperator,
    recovery_operator: Arc<dyn Recreate>,
    max_repeat_count: usize,
    alternative_objectives_probability: (Float, Float),
    skip_constraint_check_probability: (Float, Float),
}

impl InfeasibleSearch {
    /// Creates a new instance of `InfeasibleSearch`.
    pub fn new(
        inner_search: TargetSearchOperator,
        recovery_operator: Arc<dyn Recreate>,
        max_repeat_count: usize,
        shuffle_objectives_probability: (Float, Float),
        skip_constraint_check_probability: (Float, Float),
    ) -> Self {
        Self {
            inner_search,
            recovery_operator,
            max_repeat_count,
            alternative_objectives_probability: shuffle_objectives_probability,
            skip_constraint_check_probability,
        }
    }

    fn recover_individual(
        &self,
        orig_refinement_ctx: &RefinementContext,
        new_insertion_ctx: InsertionContext,
    ) -> InsertionContext {
        let new_insertion_ctx = repair_solution_from_unknown(&new_insertion_ctx, &|| {
            InsertionContext::new(orig_refinement_ctx.problem.clone(), orig_refinement_ctx.environment.clone())
        });

        // NOTE: give a chance to rearrange unassigned jobs
        let mut new_insertion_ctx = self.recovery_operator.run(orig_refinement_ctx, new_insertion_ctx);
        finalize_insertion_ctx(&mut new_insertion_ctx);

        new_insertion_ctx
    }
}

impl HeuristicSearchOperator for InfeasibleSearch {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let refinement_ctx = heuristic_ctx;
        let insertion_ctx = solution;

        let new_insertion_ctx = create_relaxed_insertion_ctx(
            insertion_ctx,
            self.alternative_objectives_probability,
            self.skip_constraint_check_probability,
        );
        let mut new_refinement_ctx = create_relaxed_refinement_ctx(&new_insertion_ctx);

        let repeat_count = refinement_ctx.environment.random.uniform_int(1, self.max_repeat_count as i32);

        let mut initial = Some(new_insertion_ctx);
        for _ in 0..repeat_count {
            // NOTE from diversity reasons, we don't want to see original solution in the population
            let new_insertion_ctx = match initial.take() {
                Some(initial) => self.inner_search.search(&new_refinement_ctx, &initial),
                _ => self.inner_search.search(&new_refinement_ctx, get_random_individual(&new_refinement_ctx)),
            };

            new_refinement_ctx.add_solution(self.recover_individual(refinement_ctx, new_insertion_ctx));
        }

        new_refinement_ctx.ranked().map(|s| s.deep_copy()).next().unwrap_or_else(|| solution.deep_copy())
    }
}

fn create_relaxed_refinement_ctx(new_insertion_ctx: &InsertionContext) -> RefinementContext {
    let problem = new_insertion_ctx.problem.clone();
    let environment = new_insertion_ctx.environment.clone();
    let population = Box::new(ElitismPopulation::new(problem.goal.clone(), environment.random.clone(), 4, 4));

    // NOTE statistic is reset to default
    RefinementContext::new(problem, population, TelemetryMode::None, environment)
}

fn create_relaxed_insertion_ctx(
    insertion_ctx: &InsertionContext,
    alt_objectives_probability: (Float, Float),
    skip_constraint_check_probability: (Float, Float),
) -> InsertionContext {
    let problem = &insertion_ctx.problem;
    let random = &insertion_ctx.environment.random;

    let alternative_prob = random.uniform_real(alt_objectives_probability.0, alt_objectives_probability.1);
    let skip_prob = random.uniform_real(skip_constraint_check_probability.0, skip_constraint_check_probability.1);

    let variant = create_modified_variant(problem.goal.as_ref(), random.clone(), skip_prob, alternative_prob);

    let mut insertion_ctx = insertion_ctx.deep_copy();
    insertion_ctx.problem = Arc::new(Problem {
        fleet: problem.fleet.clone(),
        jobs: problem.jobs.clone(),
        locks: problem.locks.clone(),
        goal: variant,
        activity: problem.activity.clone(),
        transport: problem.transport.clone(),
        extras: problem.extras.clone(),
    });

    insertion_ctx
}

fn create_modified_variant(
    original: &GoalContext,
    random: Arc<dyn Random>,
    skip_probability: Float,
    alternative_probability: Float,
) -> Arc<GoalContext> {
    let alternative =
        if random.is_hit(alternative_probability) { original.maybe_new(random.as_ref()) } else { original.clone() };

    let constraints = alternative.constraints().map(|constraint| {
        let skip_probability = if random.is_head_not_tails() { 1. } else { skip_probability };

        let value: Arc<dyn FeatureConstraint> = Arc::new(StochasticFeatureConstraint {
            inner: constraint.clone(),
            random: random.clone(),
            probability: skip_probability,
        });

        value
    });

    Arc::new(alternative.with_constraints(constraints))
}

fn get_random_individual(new_refinement_ctx: &RefinementContext) -> &InsertionContext {
    let selected = new_refinement_ctx.selected().collect::<Vec<_>>();
    let skip = new_refinement_ctx.environment.random.uniform_int(0, selected.len() as i32 - 1) as usize;

    selected.get(skip).expect("no individual")
}

struct StochasticFeatureConstraint {
    inner: Arc<dyn FeatureConstraint>,
    random: Arc<dyn Random>,
    probability: Float,
}

impl FeatureConstraint for StochasticFeatureConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        if self.random.is_hit(self.probability) { None } else { self.inner.evaluate(move_ctx) }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        self.inner.merge(source, candidate)
    }
}
