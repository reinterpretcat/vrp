use crate::construction::heuristics::*;
use crate::construction::probing::repair_solution_from_unknown;
use crate::models::problem::Job;
use crate::models::*;
use crate::solver::*;
use rosomaxa::population::Shuffled;
use std::cmp::Ordering;
use std::sync::Arc;

/// A mutation operator which performs search in infeasible space.
pub struct InfeasibleSearch {
    inner_search: TargetSearchOperator,
    repeat_count: usize,
    shuffle_objectives_probability: (f64, f64),
    skip_constraint_check_probability: (f64, f64),
}

impl InfeasibleSearch {
    /// Creates a new instance of `InfeasibleSearch`.
    pub fn new(
        inner_search: TargetSearchOperator,
        repeat_count: usize,
        shuffle_objectives_probability: (f64, f64),
        skip_constraint_check_probability: (f64, f64),
    ) -> Self {
        Self { inner_search, repeat_count, shuffle_objectives_probability, skip_constraint_check_probability }
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
            self.shuffle_objectives_probability,
            self.skip_constraint_check_probability,
        );
        let mut new_refinement_ctx = create_relaxed_refinement_ctx(&new_insertion_ctx);

        let repeat_count = refinement_ctx.environment.random.uniform_int(1, self.repeat_count as i32);

        (0..repeat_count).fold(Some(new_insertion_ctx), |initial, _| {
            // NOTE from diversity reasons, we don't want to see original solution in the population
            let new_insertion_ctx = if let Some(initial) = initial {
                self.inner_search.search(&new_refinement_ctx, &initial)
            } else {
                self.inner_search.search(&new_refinement_ctx, get_random_individual(&new_refinement_ctx))
            };

            new_refinement_ctx.add_solution(new_insertion_ctx);

            None
        });

        let new_insertion_ctx = get_best_or_random_individual(&new_refinement_ctx, insertion_ctx);

        repair_solution_from_unknown(new_insertion_ctx, &|| {
            InsertionContext::new(insertion_ctx.problem.clone(), insertion_ctx.environment.clone())
        })
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
    shuffle_objectives_probability: (f64, f64),
    skip_constraint_check_probability: (f64, f64),
) -> InsertionContext {
    let problem = &insertion_ctx.problem;
    let random = &insertion_ctx.environment.random;

    let shuffle_prob = random.uniform_real(shuffle_objectives_probability.0, shuffle_objectives_probability.1);
    let skip_prob = random.uniform_real(skip_constraint_check_probability.0, skip_constraint_check_probability.1);

    let variant = create_modified_variant(problem.goal.as_ref(), random.clone(), skip_prob, shuffle_prob);

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
    random: Arc<dyn Random + Send + Sync>,
    skip_probability: f64,
    shuffle_probability: f64,
) -> Arc<GoalContext> {
    let shuffled =
        if random.is_hit(shuffle_probability) { original.get_shuffled(random.as_ref()) } else { original.clone() };

    let constraints = shuffled
        .constraints
        .iter()
        .map(|constraint| {
            let skip_probability = if random.is_head_not_tails() { 1. } else { skip_probability };

            let value: Arc<dyn FeatureConstraint + Send + Sync> = Arc::new(StochasticFeatureConstraint {
                inner: constraint.clone(),
                random: random.clone(),
                probability: skip_probability,
            });

            value
        })
        .collect();

    Arc::new(GoalContext { constraints, ..shuffled })
}

fn get_random_individual(new_refinement_ctx: &RefinementContext) -> &InsertionContext {
    let size = new_refinement_ctx.population().size();
    let skip = new_refinement_ctx.environment.random.uniform_int(0, size as i32 - 1) as usize;

    new_refinement_ctx.population().select().nth(skip).expect("no individual")
}

fn get_best_or_random_individual<'a>(
    new_refinement_ctx: &'a RefinementContext,
    old_insertion_ctx: &InsertionContext,
) -> &'a InsertionContext {
    let new_insertion_ctx = new_refinement_ctx.population().select().next().expect("no individual");

    if new_refinement_ctx.problem.goal.total_order(new_insertion_ctx, old_insertion_ctx) == Ordering::Less {
        new_insertion_ctx
    } else {
        get_random_individual(new_refinement_ctx)
    }
}

struct StochasticFeatureConstraint {
    inner: Arc<dyn FeatureConstraint + Send + Sync>,
    random: Arc<dyn Random + Send + Sync>,
    probability: f64,
}

impl FeatureConstraint for StochasticFeatureConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        if self.random.is_hit(self.probability) {
            None
        } else {
            self.inner.evaluate(move_ctx)
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        self.inner.merge(source, candidate)
    }
}
