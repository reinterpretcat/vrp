use super::*;
use crate::construction::features::{CapacityFeatureBuilder, TransportFeatureBuilder, VehicleCapacityDimension};
use crate::helpers::construction::features::create_simple_demand;
use crate::helpers::models::domain::{TestGoalContextBuilder, get_customer_ids_from_routes};
use crate::helpers::models::problem::TestSingleBuilder;
use crate::helpers::solver::{
    create_default_refinement_ctx, generate_matrix_routes, promote_to_locked, rearrange_jobs_in_routes,
};
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use crate::models::ViolationCode;
use crate::models::common::SingleDimLoad;
use crate::models::problem::JobIdDimension;
use crate::models::{FeatureBuilder, FeatureState};
use std::sync::Arc;

struct ActivateJobWhenRouteIsRemoved {
    job: Job,
}

impl FeatureState for ActivateJobWhenRouteIsRemoved {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        if solution_ctx.routes.len() == 2 && solution_ctx.ignored.contains(&self.job) {
            solution_ctx.ignored.retain(|job| job != &self.job);
            solution_ctx.unassigned.insert(self.job.clone(), UnassignmentInfo::Unknown);
        }
    }
}

fn create_insertion_ctx(demands: &[i32], rows: usize, job_order: &[Vec<&str>]) -> InsertionContext {
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![0], vec![0.; 128])));
    let (problem, solution) = generate_matrix_routes(
        rows,
        job_order.len(),
        true,
        |transport, activity, _| {
            TestGoalContextBuilder::empty()
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
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    rearrange_jobs_in_routes(&mut insertion_ctx, job_order);

    insertion_ctx
}

fn create_default_insertion_ctx(demands: &[i32]) -> InsertionContext {
    create_insertion_ctx(demands, 2, &[vec!["c0", "c1", "c5"], vec!["c2", "c3"], vec!["c4"]])
}

#[test]
fn can_eliminate_route_using_single_job_ejection() {
    // Both remaining routes have only three units of spare capacity, so c4 cannot be inserted
    // directly. Ejecting c1 makes room for c4 and c1 can then be inserted into the other route.
    let insertion_ctx = create_default_insertion_ctx(&[5, 2, 4, 3, 4, 0]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    let result = GuidedEjectionSearch::new().search(&refinement_ctx, &insertion_ctx);
    let mut actual_jobs = get_customer_ids_from_routes(&result).into_iter().flatten().collect::<Vec<_>>();
    actual_jobs.sort();

    assert_eq!(result.solution.routes.len(), 2);
    assert!(result.solution.unassigned.is_empty());
    assert!(Arc::ptr_eq(&result.environment, &insertion_ctx.environment));
    assert_eq!(actual_jobs, (0..6).map(|idx| format!("c{idx}")).collect::<Vec<_>>());
}

#[test]
fn can_eliminate_route_using_two_job_ejection() {
    // The two remaining routes have only two units of spare capacity and contain unit-demand jobs.
    // Making room for c16 therefore requires ejecting two jobs at once; both fit in the other route.
    let demands = (0..18)
        .map(|idx| {
            if idx == 16 {
                4
            } else if idx == 17 {
                0
            } else {
                1
            }
        })
        .collect::<Vec<_>>();
    let insertion_ctx = create_insertion_ctx(
        demands.as_slice(),
        6,
        &[
            vec!["c0", "c1", "c2", "c3", "c4", "c5", "c6", "c7"],
            vec!["c8", "c9", "c10", "c11", "c12", "c13", "c14", "c15"],
            vec!["c16", "c17"],
        ],
    );
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    let result = GuidedEjectionSearch::new().search(&refinement_ctx, &insertion_ctx);

    assert_eq!(result.solution.routes.len(), 2);
    assert!(result.solution.unassigned.is_empty());
    assert_eq!(result.solution.routes.iter().map(|route| route.route().tour.job_count()).sum::<usize>(), 18);
}

#[test]
fn reinserts_unassigned_job_activated_by_route_removal() {
    let conditional = Job::Single(
        TestSingleBuilder::default().id("conditional").location(Some(0)).demand(create_simple_demand(0)).build_shared(),
    );
    let state = ActivateJobWhenRouteIsRemoved { job: conditional.clone() };
    let demands = [5, 2, 4, 3, 4, 0];
    let (problem, solution) = generate_matrix_routes(
        2,
        3,
        true,
        |transport, activity, _| {
            TestGoalContextBuilder::empty()
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
                .add_feature(FeatureBuilder::default().with_name("conditional").with_state(state).build().unwrap())
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
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![0], vec![0.; 128])));
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    rearrange_jobs_in_routes(&mut insertion_ctx, &[vec!["c0", "c1", "c5"], vec!["c2", "c3"], vec!["c4"]]);
    insertion_ctx.solution.ignored.push(conditional.clone());
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    let result = GuidedEjectionSearch::new().search(&refinement_ctx, &insertion_ctx);

    assert_eq!(result.solution.routes.len(), 2);
    assert!(result.solution.required.is_empty());
    assert!(result.solution.unassigned.is_empty());
    assert!(result.solution.routes.iter().any(|route| route.route().tour.contains(&conditional)));
}

#[test]
fn preserves_original_solution_when_route_cannot_be_eliminated() {
    let insertion_ctx = create_default_insertion_ctx(&[5, 2, 4, 3, 7, 0]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    let result = GuidedEjectionSearch::new().search(&refinement_ctx, &insertion_ctx);

    assert_eq!(get_customer_ids_from_routes(&result), get_customer_ids_from_routes(&insertion_ctx));
    assert!(result.solution.unassigned.is_empty());
}

#[test]
fn does_not_start_route_elimination_from_infeasible_incumbent() {
    let mut insertion_ctx = create_default_insertion_ctx(&[5, 2, 4, 3, 4, 0]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let job = insertion_ctx.solution.routes[0].route().tour.jobs().next().unwrap().clone();
    let search = GuidedEjectionSearch::new();

    insertion_ctx.solution.unassigned.insert(job.clone(), UnassignmentInfo::Unknown);
    assert!(search.diversify(&refinement_ctx, &insertion_ctx).is_empty());

    insertion_ctx.solution.unassigned.clear();
    insertion_ctx.solution.required.push(job);
    assert!(search.diversify(&refinement_ctx, &insertion_ctx).is_empty());
}

#[test]
fn does_not_remove_route_with_locked_jobs() {
    let insertion_ctx =
        promote_to_locked(create_default_insertion_ctx(&[5, 2, 4, 3, 4, 0]), &["c0", "c1", "c2", "c3", "c4", "c5"]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    let result = GuidedEjectionSearch::new().search(&refinement_ctx, &insertion_ctx);

    assert_eq!(get_customer_ids_from_routes(&result), get_customer_ids_from_routes(&insertion_ctx));
}

#[test]
fn skips_smallest_route_when_only_one_of_its_jobs_is_locked() {
    let insertion_ctx = promote_to_locked(create_default_insertion_ctx(&[5, 2, 4, 3, 4, 0]), &["c4"]);

    assert_eq!(select_source_route(&insertion_ctx), Some(1));
}

#[test]
fn does_not_use_locked_jobs_for_ejection() {
    let mut insertion_ctx =
        promote_to_locked(create_default_insertion_ctx(&[5, 2, 4, 3, 4, 0]), &["c0", "c1", "c2", "c3", "c5"]);
    let source = insertion_ctx.solution.routes.pop().unwrap();
    let job =
        source.route().tour.jobs().find(|job| job.dimens().get_job_id().is_some_and(|id| id == "c4")).unwrap().clone();
    insertion_ctx.problem.goal.accept_solution_state(&mut insertion_ctx.solution);
    let mut budget = EjectionEvaluationBudget::new(10);

    let ejection = find_ejection(&insertion_ctx, &job, &HashMap::new(), &mut budget);

    assert!(ejection.is_none());
}

#[test]
fn schedules_failed_attempts_less_often_than_successes() {
    let schedule = SearchSchedule::default();

    assert!(schedule.try_reserve(10));
    assert!(!schedule.try_reserve(10));
    schedule.complete(10, 100, false);
    assert!(!schedule.try_reserve(109));
    assert!(schedule.try_reserve(110));

    schedule.complete(110, 100, false);
    assert!(!schedule.try_reserve(309));
    assert!(schedule.try_reserve(310));

    schedule.complete(310, 100, false);
    assert!(!schedule.try_reserve(709));
    assert!(schedule.try_reserve(710));

    schedule.complete(710, 100, false);
    assert!(!schedule.try_reserve(1_509));
    assert!(schedule.try_reserve(1_510));

    schedule.complete(1_510, 100, true);
    assert!(!schedule.try_reserve(1_609));
    assert!(schedule.try_reserve(1_610));

    schedule.complete(1_610, 100, false);
    assert!(!schedule.try_reserve(1_709));
    assert!(schedule.try_reserve(1_710));
}

#[test]
fn eliminates_route_from_incumbent_instead_of_diverse_parent() {
    let insertion_ctx = create_default_insertion_ctx(&[5, 2, 4, 3, 4, 0]);
    let mut refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    refinement_ctx.add_solution(insertion_ctx.deep_copy());
    let diverse = promote_to_locked(insertion_ctx, &["c0", "c1", "c2", "c3", "c4", "c5"]);

    let result = GuidedEjectionSearch::new().diversify(&refinement_ctx, &diverse);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].solution.routes.len(), 2);
    assert!(result[0].solution.locked.is_empty());
}

#[test]
fn can_limit_pair_ejection_evaluations() {
    let mut budget = EjectionEvaluationBudget::new(2);

    assert!(budget.try_consume_pair());
    assert!(budget.try_consume_pair());
    assert!(!budget.try_consume_pair());
    assert!(budget.is_exhausted());
}

#[test]
fn can_use_first_feasible_ejection_penalty_tier() {
    let mut insertion_ctx = create_default_insertion_ctx(&[5, 2, 4, 3, 4, 0]);
    let source = insertion_ctx.solution.routes.pop().unwrap();
    let job =
        source.route().tour.jobs().find(|job| job.dimens().get_job_id().is_some_and(|id| id == "c4")).unwrap().clone();
    insertion_ctx.problem.goal.accept_solution_state(&mut insertion_ctx.solution);

    // Ejecting zero-demand c5 cannot make room for c4. All useful ejections have a higher penalty,
    // so the search has to advance past the cheapest, infeasible tier.
    let attempts = insertion_ctx
        .solution
        .routes
        .iter()
        .flat_map(|route| route.route().tour.jobs().cloned())
        .map(|job| {
            let penalty = usize::from(!job.dimens().get_job_id().is_some_and(|id| id == "c5"));
            (job, penalty)
        })
        .collect::<HashMap<_, _>>();
    let mut budget = EjectionEvaluationBudget::new(10);

    let ejection = find_ejection(&insertion_ctx, &job, &attempts, &mut budget).unwrap();

    assert_eq!(attempts.get(&ejection.first), Some(&1));
    assert!(ejection.second.is_none());
}

#[test]
fn returns_diverse_offspring_only_for_reserved_success() {
    let insertion_ctx = create_default_insertion_ctx(&[5, 2, 4, 3, 4, 0]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let search = GuidedEjectionSearch::new();

    assert_eq!(search.diversify(&refinement_ctx, &insertion_ctx).len(), 1);
    assert!(search.diversify(&refinement_ctx, &insertion_ctx).is_empty());
}
