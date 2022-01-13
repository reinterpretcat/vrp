use crate::construction::constraints::*;
use crate::construction::heuristics::*;
use crate::construction::probing::repair_solution_from_unknown;
use crate::models::problem::Job;
use crate::models::Problem;
use crate::solver::*;
use rosomaxa::population::Shuffled;
use std::cmp::Ordering;
use std::sync::Arc;

/// A mutation operator which performs search in infeasible space.
pub struct InfeasibleSearch {
    inner_search: TargetHeuristicOperator,
    repeat_count: usize,
    shuffle_objectives_probability: (f64, f64),
    skip_constraint_check_probability: (f64, f64),
}

impl InfeasibleSearch {
    /// Creates a new instance of `InfeasibleSearch`.
    pub fn new(
        inner_search: TargetHeuristicOperator,
        repeat_count: usize,
        shuffle_objectives_probability: (f64, f64),
        skip_constraint_check_probability: (f64, f64),
    ) -> Self {
        Self { inner_search, repeat_count, shuffle_objectives_probability, skip_constraint_check_probability }
    }
}

impl HeuristicOperator for InfeasibleSearch {
    type Context = RefinementContext;
    type Objective = ProblemObjective;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let refinement_ctx = heuristic_ctx;
        let insertion_ctx = solution;

        let new_insertion_ctx = create_relaxed_insertion_ctx(
            insertion_ctx,
            self.shuffle_objectives_probability,
            self.skip_constraint_check_probability,
        );
        let mut new_refinement_ctx = create_relaxed_refinement_ctx(refinement_ctx, &new_insertion_ctx);

        let repeat_count = refinement_ctx.environment.random.uniform_int(1, self.repeat_count as i32);

        (0..repeat_count).fold(Some(new_insertion_ctx), |initial, _| {
            // NOTE from diversity reasons, we don't want to see original solution in the population
            let new_insertion_ctx = if let Some(initial) = initial {
                self.inner_search.search(&new_refinement_ctx, &initial)
            } else {
                self.inner_search.search(&new_refinement_ctx, get_random_individual(&new_refinement_ctx))
            };

            new_refinement_ctx.population.add(new_insertion_ctx);

            None
        });

        let new_insertion_ctx = get_best_or_random_individual(&new_refinement_ctx, insertion_ctx);

        repair_solution_from_unknown(new_insertion_ctx, &|| {
            InsertionContext::new(insertion_ctx.problem.clone(), insertion_ctx.environment.clone())
        })
    }
}

fn create_relaxed_refinement_ctx(
    refinement_ctx: &RefinementContext,
    new_insertion_ctx: &InsertionContext,
) -> RefinementContext {
    let problem = new_insertion_ctx.problem.clone();
    let environment = new_insertion_ctx.environment.clone();
    let population = Box::new(ElitismPopulation::new(problem.objective.clone(), environment.random.clone(), 4, 4));

    RefinementContext {
        problem,
        population,
        state: Default::default(),
        environment,
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
        Arc::new(create_modified_constraint(problem.constraint.as_ref(), random.clone(), skip_prob))
    } else {
        problem.constraint.clone()
    };

    let objective = if random.is_hit(shuffle_prob) {
        Arc::new(problem.objective.get_shuffled(random.as_ref()))
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

fn create_modified_constraint(
    original: &ConstraintPipeline,
    random: Arc<dyn Random + Send + Sync>,
    skip_probability: f64,
) -> ConstraintPipeline {
    let mut pipeline = ConstraintPipeline {
        modules: original.modules.clone(),
        state_keys: original.state_keys.clone(),
        ..ConstraintPipeline::default()
    };

    if random.is_head_not_tails() {
        use_stochastic_rule(original, &mut pipeline, random, skip_probability);
    } else {
        use_permissive_rule(original, &mut pipeline, random);
    }

    pipeline
}

fn use_stochastic_rule(
    original: &ConstraintPipeline,
    modified: &mut ConstraintPipeline,
    random: Arc<dyn Random + Send + Sync>,
    skip_probability: f64,
) {
    original.get_constraints().for_each(|constraint| {
        let constraint: ConstraintVariant =
            match constraint {
                ConstraintVariant::HardRoute(c) => ConstraintVariant::HardRoute(Arc::new(
                    StochasticHardConstraint::new(Some(c), None, random.clone(), skip_probability),
                )),
                ConstraintVariant::HardActivity(c) => ConstraintVariant::HardActivity(Arc::new(
                    StochasticHardConstraint::new(None, Some(c), random.clone(), skip_probability),
                )),
                _ => constraint,
            };
        modified.add_constraint(&constraint);
    });
}

fn use_permissive_rule(
    original: &ConstraintPipeline,
    modified: &mut ConstraintPipeline,
    random: Arc<dyn Random + Send + Sync>,
) {
    let constraints = original
        .modules
        .iter()
        .map(|module| {
            module
                .get_constraints()
                .map(|constraint| match &constraint {
                    ConstraintVariant::HardRoute(_) | ConstraintVariant::HardActivity(_) => (constraint, true),
                    _ => (constraint, false),
                })
                .collect::<Vec<_>>()
        })
        .filter(|constraints| !constraints.is_empty())
        .collect::<Vec<_>>();

    let indices = constraints
        .iter()
        .enumerate()
        // NOTE as permissive rule, we just skip constraint entirely
        .filter(|(_, constraints)| constraints.iter().any(|(_, is_hard)| *is_hard))
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();

    assert!(!indices.is_empty());

    let skip_index = random.uniform_int(0, indices.len() as i32 - 1) as usize;

    constraints
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != indices[skip_index])
        .flat_map(|(_, constraints)| constraints.iter())
        .for_each(|(constraint, _)| modified.add_constraint(constraint));
}

fn get_random_individual(new_refinement_ctx: &RefinementContext) -> &InsertionContext {
    let size = new_refinement_ctx.population.size();
    let skip = new_refinement_ctx.environment.random.uniform_int(0, size as i32 - 1) as usize;

    new_refinement_ctx.population.select().nth(skip).expect("no individual")
}

fn get_best_or_random_individual<'a>(
    new_refinement_ctx: &'a RefinementContext,
    old_insertion_ctx: &InsertionContext,
) -> &'a InsertionContext {
    let new_insertion_ctx = new_refinement_ctx.population.select().next().expect("no individual");

    if new_refinement_ctx.problem.objective.total_order(new_insertion_ctx, old_insertion_ctx) == Ordering::Less {
        new_insertion_ctx
    } else {
        get_random_individual(new_refinement_ctx)
    }
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
