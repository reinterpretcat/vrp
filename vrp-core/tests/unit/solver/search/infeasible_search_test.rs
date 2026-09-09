use super::*;
use crate::construction::features::{CapacityFeatureBuilder, TransportFeatureBuilder, VehicleCapacityDimension};
use crate::helpers::construction::features::create_simple_demand;
use crate::helpers::models::domain::{TestGoalContextBuilder, get_customer_ids_from_routes};
use crate::helpers::models::problem::TestSingleBuilder;
use crate::helpers::solver::{generate_matrix_routes, rearrange_jobs_in_routes};
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::{Footprint, SingleDimLoad};
use crate::models::problem::{JobIdDimension, VehicleIdDimension};
use crate::models::{Lock, LockDetail, LockOrder, LockPosition, Problem, ViolationCode};
use crate::solver::search::RecreateWithCheapest;
use rosomaxa::evolution::TelemetryMode;
use rosomaxa::population::{HeuristicPopulation, Rosomaxa, RosomaxaConfig, RosomaxaSolution};
use rosomaxa::utils::DefaultRandom;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};

custom_solution_state!(TestCost typeof Float);

struct TestCostObjective;

impl FeatureObjective for TestCostObjective {
    fn fitness(&self, solution: &InsertionContext) -> Float {
        solution.solution.state.get_test_cost().copied().unwrap_or_default()
    }

    fn estimate(&self, _: &MoveContext<'_>) -> Float {
        0.
    }
}

#[test]
fn can_choose_relaxed_search_transition() {
    let epsilon = Float::EPSILON.sqrt();

    assert_eq!(get_next_step(false, 0.), RelaxedSearchStep::Restore);
    assert_eq!(get_next_step(true, epsilon), RelaxedSearchStep::Repair);
    assert_eq!(get_next_step(false, epsilon), RelaxedSearchStep::Seed);
}

#[test]
fn can_contract_infeasibility_tolerance() {
    assert_eq!(get_infeasibility_tolerance(None), 0.05);
    assert_eq!(get_infeasibility_tolerance(Some(0.08)), 0.04);
}

#[test]
fn can_accept_repair_according_to_population_phase() {
    assert!(is_repair_improvement(SelectionPhase::Exploration, Ordering::Less, Some(Ordering::Greater)));
    assert!(!is_repair_improvement(SelectionPhase::Exploration, Ordering::Greater, Some(Ordering::Less)));
    assert!(is_repair_improvement(SelectionPhase::Exploitation, Ordering::Less, Some(Ordering::Less)));
    assert!(!is_repair_improvement(SelectionPhase::Exploitation, Ordering::Less, Some(Ordering::Greater)));
}

#[test]
fn can_apply_configured_alternative_probability_once() {
    let random = Arc::new(FakeRandom::new(vec![0], vec![0.2, 0.]));
    let insertion_ctx = create_capacity_insertion_ctx_with_random(random);

    assert!(create_relaxed_insertion_ctx(&insertion_ctx, (0.05, 0.2)).is_some());
}

struct CreateCapacityOverload;

impl HeuristicSearchOperator for CreateCapacityOverload {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let mut candidate = solution.deep_copy();
        if !candidate.problem.goal.is_relaxed() {
            return candidate;
        }

        rearrange_jobs_in_routes(&mut candidate, &[vec![], vec!["c1", "c2", "c3", "c0"]]);
        assert!(get_violation(&candidate).is_some_and(|violation| violation > 0.));
        candidate
    }
}

struct ImproveOnlyUnderStrictGoal {
    strict_calls: Arc<AtomicUsize>,
    relaxed_calls: Arc<AtomicUsize>,
}

impl HeuristicSearchOperator for ImproveOnlyUnderStrictGoal {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let mut candidate = solution.deep_copy();

        if candidate.problem.goal.is_relaxed() {
            self.relaxed_calls.fetch_add(1, AtomicOrdering::Relaxed);
        } else {
            self.strict_calls.fetch_add(1, AtomicOrdering::Relaxed);
            candidate.solution.state.set_test_cost(9.);
        }

        candidate
    }
}

struct CreateInfeasibleAndFeasibleSiblings {
    relaxed_calls: AtomicUsize,
}

impl HeuristicSearchOperator for CreateInfeasibleAndFeasibleSiblings {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let mut candidate = solution.deep_copy();

        if !candidate.problem.goal.is_relaxed() {
            return candidate;
        }

        match self.relaxed_calls.fetch_add(1, AtomicOrdering::Relaxed) {
            0 => {
                rearrange_jobs_in_routes(&mut candidate, &[vec![], vec!["c1", "c2", "c3", "c0"]]);
                candidate.solution.state.set_test_cost(5.);
            }
            _ => {
                rearrange_jobs_in_routes(&mut candidate, &[vec!["c0"], vec!["c1", "c2", "c3"]]);
                candidate.solution.state.set_test_cost(9.);
            }
        }

        candidate
    }
}

struct CheckStrictRecovery {
    inner: RecreateWithCheapest,
    called: Arc<AtomicBool>,
}

impl Recreate for CheckStrictRecovery {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let insertion_ctx = self.inner.run(refinement_ctx, insertion_ctx);

        assert!(!insertion_ctx.problem.goal.is_relaxed());
        assert!(insertion_ctx.solution.state.get_relaxed_violation().is_none());
        assert!(insertion_ctx.solution.locked.iter().any(|job| job.dimens().get_job_id().is_some_and(|id| id == "c1")));
        assert_registry_consistent(&insertion_ctx);
        self.called.store(true, AtomicOrdering::Relaxed);

        insertion_ctx
    }
}

#[test]
fn can_keep_relaxed_search_isolated_and_recover_with_strict_invariants() {
    let mut insertion_ctx = create_capacity_insertion_ctx();
    let mut refinement_ctx = create_refinement_ctx(&mut insertion_ctx);
    let recovery_called = Arc::new(AtomicBool::new(false));
    let search = InfeasibleSearch::new(
        Arc::new(CreateCapacityOverload),
        Arc::new(CheckStrictRecovery {
            inner: RecreateWithCheapest::new(insertion_ctx.environment.random.clone()),
            called: recovery_called.clone(),
        }),
        1,
        (0.05, 0.2),
    );

    let relaxed = search.search(&refinement_ctx, &insertion_ctx);
    assert!(get_violation(&relaxed).is_some_and(|violation| violation > 0.));
    assert_eq!(get_locked_route(&relaxed, "c1"), get_locked_route(&insertion_ctx, "c1"));
    assert_registry_consistent(&relaxed);

    refinement_ctx.add_solution(relaxed);
    let result = search.search(&refinement_ctx, &insertion_ctx);

    assert!(recovery_called.load(AtomicOrdering::Relaxed));
    assert_eq!(get_locked_route(&result, "c1"), get_locked_route(&insertion_ctx, "c1"));
    assert_registry_consistent(&result);
    assert!(refinement_ctx.ranked().all(|solution| solution.solution.state.get_relaxed_violation().is_none()));
}

#[test]
fn can_skip_relaxation_when_strict_education_improves() {
    let insertion_ctx = create_cost_capacity_insertion_ctx(Arc::new(FakeRandom::new(vec![1], vec![])));
    let refinement_ctx = create_elitism_refinement_ctx(&insertion_ctx);
    let strict_calls = Arc::new(AtomicUsize::new(0));
    let relaxed_calls = Arc::new(AtomicUsize::new(0));
    let search = InfeasibleSearch::new(
        Arc::new(ImproveOnlyUnderStrictGoal {
            strict_calls: strict_calls.clone(),
            relaxed_calls: relaxed_calls.clone(),
        }),
        Arc::new(RecreateWithCheapest::new(insertion_ctx.environment.random.clone())),
        1,
        (0.05, 0.2),
    );

    let result = search.search(&refinement_ctx, &insertion_ctx);

    assert_eq!(result.solution.state.get_test_cost(), Some(&9.));
    assert_eq!(strict_calls.load(AtomicOrdering::Relaxed), 1);
    assert_eq!(relaxed_calls.load(AtomicOrdering::Relaxed), 0);
    assert!(result.solution.state.get_relaxed_violation().is_none());
}

#[test]
fn can_return_improved_feasible_sibling_before_better_ranked_infeasible_one() {
    let random = Arc::new(FakeRandom::new(vec![3, 0, 0, 0, 0], vec![0.05, 1.]));
    let insertion_ctx = create_cost_capacity_insertion_ctx(random);
    let refinement_ctx = create_elitism_refinement_ctx(&insertion_ctx);
    let search = InfeasibleSearch::new(
        Arc::new(CreateInfeasibleAndFeasibleSiblings { relaxed_calls: AtomicUsize::new(0) }),
        Arc::new(RecreateWithCheapest::new(insertion_ctx.environment.random.clone())),
        3,
        (0.05, 0.2),
    );

    let result = search.search(&refinement_ctx, &insertion_ctx);

    assert_eq!(result.solution.state.get_test_cost(), Some(&9.));
    assert!(result.solution.state.get_relaxed_violation().is_none());
    assert_registry_consistent(&result);
}

fn create_capacity_insertion_ctx() -> InsertionContext {
    create_capacity_insertion_ctx_with_random(Arc::new(DefaultRandom::default()))
}

fn create_capacity_insertion_ctx_with_random(random: Arc<dyn Random>) -> InsertionContext {
    create_capacity_insertion_ctx_with_options(random, false)
}

fn create_cost_capacity_insertion_ctx(random: Arc<dyn Random>) -> InsertionContext {
    let mut insertion_ctx = create_capacity_insertion_ctx_with_options(random, true);
    insertion_ctx.solution.state.set_test_cost(10.);

    insertion_ctx
}

fn create_capacity_insertion_ctx_with_options(random: Arc<dyn Random>, with_test_cost: bool) -> InsertionContext {
    let environment = create_test_environment_with_random(random);
    let demands = [4, 4, 3, 0];
    let (problem, solution) = generate_matrix_routes(
        2,
        2,
        true,
        |transport, activity, _| {
            let builder = TestGoalContextBuilder::empty();
            let builder = if with_test_cost {
                builder.add_feature(
                    FeatureBuilder::default().with_name("test_cost").with_objective(TestCostObjective).build().unwrap(),
                )
            } else {
                builder
            };

            builder
                .add_feature(
                    TransportFeatureBuilder::new("transport")
                        .set_violation_code(ViolationCode(1))
                        .set_transport_cost(transport)
                        .set_activity_cost(activity)
                        .build_minimize_cost()
                        .unwrap(),
                )
                .add_feature(
                    CapacityFeatureBuilder::<SingleDimLoad>::new("capacity")
                        .set_violation_code(ViolationCode(2))
                        .build()
                        .unwrap(),
                )
                .build()
        },
        |id, location| {
            let index = id.trim_start_matches('c').parse::<usize>().unwrap();
            TestSingleBuilder::default()
                .id(id)
                .location(location)
                .demand(create_simple_demand(demands[index]))
                .build_shared()
        },
        |mut vehicle| {
            vehicle.dimens.set_vehicle_capacity(SingleDimLoad::new(10));
            vehicle
        },
        |data| (data.clone(), data),
    );
    let locked_job =
        problem.jobs.all().iter().find(|job| job.dimens().get_job_id().is_some_and(|id| id == "c1")).unwrap().clone();
    let problem = Problem {
        locks: vec![Arc::new(Lock {
            condition_fn: Arc::new(|actor| actor.vehicle.dimens.get_vehicle_id().is_some_and(|id| id == "1")),
            details: vec![LockDetail::new(LockOrder::Strict, LockPosition::Any, vec![locked_job])],
            is_lazy: false,
        })],
        ..problem
    };
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    rearrange_jobs_in_routes(&mut insertion_ctx, &[vec!["c0"], vec!["c1", "c2", "c3"]]);

    insertion_ctx
}

fn create_refinement_ctx(insertion_ctx: &mut InsertionContext) -> RefinementContext {
    let environment = insertion_ctx.environment.clone();
    let problem = insertion_ctx.problem.clone();
    assert!(problem.goal.has_relaxations());
    let footprint = Footprint::new(problem.as_ref());
    insertion_ctx.on_init(&footprint);
    let mut population = Rosomaxa::new(
        footprint,
        problem.goal.clone(),
        environment.clone(),
        RosomaxaConfig { initial_size: 1, ..RosomaxaConfig::new_with_defaults(2) },
    )
    .unwrap();
    population.add(insertion_ctx.deep_copy());
    population.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });

    RefinementContext::new(problem, Box::new(population), TelemetryMode::None, environment)
}

fn create_elitism_refinement_ctx(insertion_ctx: &InsertionContext) -> RefinementContext {
    let problem = insertion_ctx.problem.clone();
    let environment = insertion_ctx.environment.clone();
    let mut population = ElitismPopulation::new(problem.goal.clone(), environment.random.clone(), 4, 4);
    population.add(insertion_ctx.deep_copy());

    RefinementContext::new(problem, Box::new(population), TelemetryMode::None, environment)
}

fn get_locked_route(insertion_ctx: &InsertionContext, job_id: &str) -> usize {
    get_customer_ids_from_routes(insertion_ctx)
        .iter()
        .position(|route| route.iter().any(|id| id == job_id))
        .expect("locked job is assigned")
}

fn assert_registry_consistent(insertion_ctx: &InsertionContext) {
    let registry = insertion_ctx.solution.registry.resources();
    let available = registry.available().collect::<HashSet<_>>();
    let used =
        insertion_ctx.solution.routes.iter().map(|route_ctx| route_ctx.route().actor.clone()).collect::<HashSet<_>>();

    assert!(used.is_disjoint(&available));
    assert_eq!(registry.all().count(), used.len() + available.len());
}
