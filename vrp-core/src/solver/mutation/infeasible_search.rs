use crate::construction::constraints::*;
use crate::construction::heuristics::*;
use crate::construction::probing::repair_solution_from_unknown;
use crate::models::problem::Job;
use crate::models::Problem;
use crate::solver::mutation::Mutation;
use crate::solver::population::Elitism;
use crate::solver::RefinementContext;
use crate::utils::Random;
use std::sync::Arc;

/// A mutation operator which performs search in infeasible space.
pub struct InfeasibleSearch {
    inner_mutation: Arc<dyn Mutation + Send + Sync>,
    repeat_count: usize,
    shuffle_objectives_probability: (f64, f64),
    skip_constraint_check_probability: (f64, f64),
}

impl InfeasibleSearch {
    /// Creates a new instance of `InfeasibleSearch`.
    pub fn new(
        inner_mutation: Arc<dyn Mutation + Send + Sync>,
        repeat_count: usize,
        shuffle_objectives_probability: (f64, f64),
        skip_constraint_check_probability: (f64, f64),
    ) -> Self {
        Self { inner_mutation, repeat_count, shuffle_objectives_probability, skip_constraint_check_probability }
    }
}

impl Mutation for InfeasibleSearch {
    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext {
        let new_insertion_ctx = create_relaxed_insertion_ctx(
            insertion_ctx,
            self.shuffle_objectives_probability,
            self.skip_constraint_check_probability,
        );
        let mut new_refinement_ctx = create_relaxed_refinement_ctx(refinement_ctx);

        (0..self.repeat_count).fold(Some(new_insertion_ctx), |initial, _| {
            // NOTE from diversity reasons, we don't want to see original solution in population
            let new_insertion_ctx = if let Some(initial) = initial {
                self.inner_mutation.mutate(&new_refinement_ctx, &initial)
            } else {
                let size = new_refinement_ctx.population.size();
                let skip = insertion_ctx.environment.random.uniform_int(0, size as i32 - 1) as usize;
                let new_insertion_ctx =
                    new_refinement_ctx.population.select().skip(skip).next().expect("no individual");

                self.inner_mutation.mutate(&new_refinement_ctx, &new_insertion_ctx)
            };

            new_refinement_ctx.population.add(new_insertion_ctx);

            None
        });

        let new_insertion_ctx = new_refinement_ctx.population.select().next().expect("no individual");

        repair_solution_from_unknown(new_insertion_ctx, &|| {
            InsertionContext::new(insertion_ctx.problem.clone(), insertion_ctx.environment.clone())
        })
    }
}

fn create_relaxed_refinement_ctx(refinement_ctx: &RefinementContext) -> RefinementContext {
    let problem = refinement_ctx.problem.clone();
    let population = Box::new(Elitism::new(problem.clone(), refinement_ctx.environment.random.clone(), 4, 4));

    RefinementContext {
        problem,
        population,
        state: Default::default(),
        quota: refinement_ctx.quota.clone(),
        environment: refinement_ctx.environment.clone(),
        statistics: refinement_ctx.statistics.clone(),
    }
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

    let constraint = if random.is_hit(skip_prob) {
        Arc::new(create_wrapped_constraint(problem.constraint.as_ref(), random.clone(), skip_prob))
    } else {
        problem.constraint.clone()
    };

    let objective = if random.is_hit(shuffle_prob) {
        Arc::new(problem.objective.shuffled(random.as_ref()))
    } else {
        problem.objective.clone()
    };

    let mut insertion_ctx = insertion_ctx.deep_copy();
    insertion_ctx.problem = Arc::new(Problem {
        fleet: problem.fleet.clone(),
        jobs: problem.jobs.clone(),
        locks: problem.locks.clone(),
        constraint,
        activity: problem.activity.clone(),
        transport: problem.transport.clone(),
        objective,
        extras: problem.extras.clone(),
    });

    insertion_ctx
}

fn create_wrapped_constraint(
    original: &ConstraintPipeline,
    random: Arc<dyn Random + Send + Sync>,
    skip_probability: f64,
) -> ConstraintPipeline {
    original.copy_with_modifier(&|constraint| match constraint {
        ConstraintVariant::HardRoute(c) => ConstraintVariant::HardRoute(Arc::new(StochasticHardConstraint::new(
            Some(c),
            None,
            random.clone(),
            skip_probability,
        ))),
        ConstraintVariant::HardActivity(c) => ConstraintVariant::HardActivity(Arc::new(StochasticHardConstraint::new(
            None,
            Some(c),
            random.clone(),
            skip_probability,
        ))),
        _ => constraint,
    })
}

struct StochasticHardConstraint {
    hard_route_inner: Option<Arc<dyn HardRouteConstraint + Send + Sync>>,
    hard_activity_inner: Option<Arc<dyn HardActivityConstraint + Send + Sync>>,
    random: Arc<dyn Random + Send + Sync>,
    probability: f64,
}

impl StochasticHardConstraint {
    pub fn new(
        hard_route_inner: Option<Arc<dyn HardRouteConstraint + Send + Sync>>,
        hard_activity_inner: Option<Arc<dyn HardActivityConstraint + Send + Sync>>,
        random: Arc<dyn Random + Send + Sync>,
        probability: f64,
    ) -> Self {
        Self { hard_route_inner, hard_activity_inner, random, probability }
    }
}

impl HardRouteConstraint for StochasticHardConstraint {
    fn evaluate_job(
        &self,
        solution_ctx: &SolutionContext,
        ctx: &RouteContext,
        job: &Job,
    ) -> Option<RouteConstraintViolation> {
        if self.random.is_hit(self.probability) {
            None
        } else {
            self.hard_route_inner.as_ref().unwrap().evaluate_job(solution_ctx, ctx, job)
        }
    }
}

impl HardActivityConstraint for StochasticHardConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        if self.random.is_hit(self.probability) {
            None
        } else {
            self.hard_activity_inner.as_ref().unwrap().evaluate_activity(route_ctx, activity_ctx)
        }
    }
}
