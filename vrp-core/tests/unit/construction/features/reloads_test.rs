use self::ActivityType::*;
use super::*;
use crate::construction::enablers::get_route_intervals;
use crate::construction::features::MinimizeUnassignedBuilder;
use crate::helpers::construction::features::{create_simple_demand, single_demand_as_multi};
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder};
use crate::models::problem::{JobIdDimension, VehicleIdDimension};
use crate::models::solution::Activity;
use crate::prelude::Fleet;

const VIOLATION_CODE: ViolationCode = ViolationCode(1);

struct VehicleIdDimenKey;
struct JobTypeDimenKey;
struct ResourceIdDimenKey;

fn belongs_to_route(route: &Route, job: &Job) -> bool {
    job.as_single()
        .filter(|single| is_reload_single(single.as_ref()))
        .and_then(|single| single.dimens.get_value::<VehicleIdDimenKey, String>())
        .zip(route.actor.vehicle.dimens.get_vehicle_id())
        .map_or(false, |(a, b)| a == b)
}

fn is_reload_single(single: &Single) -> bool {
    single.dimens.get_value::<JobTypeDimenKey, String>().map_or(false, |job_type| job_type == "reload")
}

fn create_simple_reload_feature<T, F>(load_schedule_threshold: F) -> Feature
where
    T: LoadOps,
    F: Fn(&T) -> T + Send + Sync + 'static,
{
    ReloadFeatureFactory::new("reload")
        .set_capacity_code(VIOLATION_CODE)
        .set_belongs_to_route(belongs_to_route)
        .set_is_reload_single(is_reload_single)
        .set_load_schedule_threshold(load_schedule_threshold)
        .build_simple()
        .expect("cannot create feature")
}

fn create_activity_with_demand(
    job_id: &str,
    pickup: (i32, i32),
    delivery: (i32, i32),
    activity_type: &str,
) -> Activity {
    ActivityBuilder::default()
        .job(Some(
            TestSingleBuilder::default()
                .id(job_id)
                .demand(single_demand_as_multi(pickup, delivery))
                .property::<JobTypeDimenKey, _>(activity_type.to_string())
                .build_shared(),
        ))
        .build()
}

fn pickup(job_id: &str, demand: (i32, i32)) -> Activity {
    create_activity_with_demand(job_id, demand, (0, 0), "pickup")
}

fn delivery(job_id: &str, demand: (i32, i32)) -> Activity {
    create_activity_with_demand(job_id, (0, 0), demand, "delivery")
}

fn reload(reload_id: &str) -> Activity {
    ActivityBuilder::default()
        .job(Some(
            TestSingleBuilder::default()
                .id(reload_id)
                .property::<JobTypeDimenKey, _>("reload".to_string())
                .property::<VehicleIdDimenKey, _>("v1".to_string())
                .build_shared(),
        ))
        .build()
}

fn create_route_context(capacity: Vec<i32>, activities: Vec<Activity>) -> RouteContext {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(TestVehicleBuilder::default().id("v1").capacity_mult(capacity).build())
        .build();

    RouteContextBuilder::default()
        .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").add_activities(activities).build())
        .build()
}

#[test]
fn can_handle_reload_jobs_with_merge() {
    let create_reload_job = || Job::Single(reload("reload").job.unwrap());
    let create_job = || TestSingleBuilder::default().location(None).build_as_job_ref();
    let feature = create_simple_reload_feature(|_| SingleDimLoad::default());
    let constraint = feature.constraint.unwrap();

    assert_eq!(constraint.merge(create_reload_job(), create_job()).map(|_| ()), Err(VIOLATION_CODE));
    assert_eq!(constraint.merge(create_job(), create_reload_job()).map(|_| ()), Err(VIOLATION_CODE));
    assert_eq!(constraint.merge(create_reload_job(), create_reload_job()).map(|_| ()), Err(VIOLATION_CODE));
}

parameterized_test! {can_remove_trivial_reloads_when_used_from_capacity_constraint, (activities, capacity, expected), {
    can_remove_trivial_reloads_when_used_from_capacity_constraint_impl(activities, capacity, expected);
}}

can_remove_trivial_reloads_when_used_from_capacity_constraint! {
    case01_no_reloads: (vec![delivery("d1", (1, 0))], 2, vec!["d1"]),
    case02_remove_at_start: (
        vec![reload("r1"), delivery("d1", (1, 0))],
        2, vec!["d1"]
    ),
    case03_remove_at_start: (
        vec![reload("r1"), reload("r2"), delivery("d1", (1, 0))],
        2, vec!["d1"]
    ),
    case04_remove_at_end: (
        vec![delivery("d1", (1, 0)), reload("r1")],
        2, vec!["d1"]
    ),
    case05_remove_at_end: (
        vec![delivery("d1", (1, 0)), reload("r1"), reload("r2")],
        2, vec!["d1"]
    ),

    case06_keep_static_delivery: (
        vec![delivery("d1", (1, 0)), delivery("d2", (1, 0)), reload("r1"), delivery("d3", (1, 0))],
        2, vec!["d1", "d2", "r1", "d3"]
    ),
    case07_keep_static_delivery: (
        vec![delivery("d1", (2, 0)), reload("r1"), delivery("d2", (1, 0))],
        2, vec!["d1", "r1", "d2"]
    ),
    case08_keep_static_delivery: (
        vec![delivery("d1", (2, 0)), reload("r1"), delivery("d2", (2, 0))],
        3, vec!["d1", "r1", "d2"]
    ),

    case09_remove_static_delivery: (
        vec![delivery("d1", (2, 0)), reload("r1"), delivery("d2", (1, 0))],
        3, vec!["d1", "d2"]
    ),
    case10_remove_static_delivery: (
        vec![delivery("d1", (1, 0)),  delivery("d2", (1, 0)), reload("r1"), delivery("d3", (1, 0))],
        4, vec!["d1", "d2", "d3"]
    ),

    case11_remove_static_pickup: (
        vec![pickup("p1", (1, 0)), pickup("p2", (1, 0)), reload("r1"), pickup("p3", (1, 0))],
        3, vec!["p1", "p2", "p3"]
    ),

    case12_keep_static_mixed: (
        vec![ pickup("p1", (1, 0)), delivery("d1", (2, 0)), reload("r1"), delivery("d2", (1, 0))],
        3, vec!["p1", "d1", "r1", "d2"]
    ),

    case13_keep_static_pickup: (
        vec![pickup("p1", (1, 0)), pickup("p2", (1, 0)), reload("r1"), pickup("p3", (1, 0))],
        2, vec!["p1", "p2", "r1", "p3"]
    ),

    case14_remove_static_mixed: (
        vec![delivery("d1", (2, 0)), reload("r1"), delivery("d2", (1, 0)), pickup("p1", (1, 0))],
        3, vec!["d1", "d2", "p1"]
    ),
    case15_remove_static_mixed: (
        vec![delivery("d1", (2, 0)), pickup("p1", (1, 0)), reload("r1"), delivery("d2", (1, 0))],
        3, vec!["d1", "p1", "d2"]
    ),

    case16_keep_multiple: (
        vec![pickup("p1", (2, 0)), reload("r1"), pickup("p2", (1, 0)), delivery("d2", (1, 0)), reload("r2"), delivery("d3", (2, 0))],
        3, vec!["p1", "r1", "p2", "d2", "r2", "d3"]
    ),

    case17_remove_multiple: (
        vec![delivery("d1", (1, 0)), reload("r1"), delivery("d2", (1, 0)), reload("r2"), delivery("d3", (1, 0)), reload("r3")],
        3, vec!["d1", "d2", "d3"]
    ),
    case18_remove_multiple: (
        vec![delivery("d1", (1, 0)), reload("r1"),  reload("r2"),  reload("r3"), delivery("d2", (1, 0))],
        2, vec!["d1", "d2"]
    ),
    case19_remove_multiple: (
        vec![delivery("d1", (1, 0)), delivery("d2", (1, 0)), reload("r1"), reload("r2"), delivery("d3", (1, 0))],
        2, vec!["d1", "d2", "r2", "d3"]
    ),
    case20_remove_multiple: (
        vec![delivery("d1", (1, 0)), delivery("d2", (1, 0)), reload("r1"), reload("r2"), reload("r3"), delivery("d3", (1, 0))],
        2, vec!["d1", "d2", "r3", "d3"]
    ),
    case21_remove_multiple: (
        vec![delivery("d1", (2, 0)), reload("r1"), pickup("p1", (1, 0)), delivery("d2", (1, 0)), reload("r2"), delivery("d3", (2, 0))],
        3, vec!["d1", "p1", "d2", "r2", "d3"]
    ),

    case22_remove_dynamic: (
        vec![pickup("p1", (0, 1)), pickup("p2", (0, 1)), reload("r1"), delivery("d1", (0, 1)), delivery("d2", (0, 1))],
        2, vec!["p1", "p2", "d1", "d2"]
    ),

    case23_remove_mixed: (
        vec![pickup("p1", (0, 1)), pickup("p2", (1, 0)), reload("r1"), delivery("d1", (0, 1))],
        2, vec!["p1", "p2", "d1"]
    ),
    case24_remove_mixed: (
        vec![delivery("d1", (1, 0)), pickup("p1", (0, 1)), reload("r1"), delivery("d2", (0, 1))],
        1, vec!["d1", "p1", "d2"]
    ),

    case25_keep_mixed: (
        vec![pickup("p1", (1, 0)), pickup("p2", (0, 1)), reload("r1"), delivery("d1", (1, 0)), delivery("d2", (0, 1))],
        2, vec!["p1", "p2", "r1", "d1", "d2"]
    ),
}

fn can_remove_trivial_reloads_when_used_from_capacity_constraint_impl(
    activities: Vec<Activity>,
    capacity: i32,
    expected: Vec<&str>,
) {
    let threshold = 0.9;
    let mut solution_ctx = TestInsertionContextBuilder::default()
        .with_routes(vec![create_route_context(vec![capacity], activities)])
        .build()
        .solution;
    let reload_feature = create_simple_reload_feature::<MultiDimLoad, _>(move |capacity| *capacity * threshold);

    let min_jobs_feature = MinimizeUnassignedBuilder::new("min_jobs").build().unwrap();
    let features = vec![reload_feature, min_jobs_feature];
    let goal = GoalContextBuilder::with_features(&features).unwrap().build().unwrap();

    goal.accept_route_state(solution_ctx.routes.get_mut(0).unwrap());
    goal.accept_solution_state(&mut solution_ctx);

    assert_eq!(
        solution_ctx
            .routes
            .first()
            .unwrap()
            .route()
            .tour
            .all_activities()
            .filter_map(|activity| activity.job.as_ref())
            .filter_map(|job| job.dimens.get_job_id())
            .collect::<Vec<_>>(),
        expected
    );
}

// shared reload

fn create_usage_activity(demand: i32) -> Activity {
    let demand = create_simple_demand(-demand);
    let single = TestSingleBuilder::default().demand(demand).build_shared();

    Activity { job: Some(single), ..ActivityBuilder::default().build() }
}

fn create_resource_activity(vehicle_id: &str, capacity: i32, resource_id: Option<SharedResourceId>) -> Activity {
    let mut builder = TestSingleBuilder::default();

    builder
        .property::<JobTypeDimenKey, _>("reload".to_string())
        .property::<VehicleIdDimenKey, _>(vehicle_id.to_string());

    if let Some(resource_id) = resource_id {
        builder.property::<ResourceIdDimenKey, _>(resource_id);
    }

    builder.dimens_mut().set_vehicle_capacity(SingleDimLoad::new(capacity));

    ActivityBuilder::default().job(Some(builder.build_shared())).build()
}

fn create_shared_reload_builder(total_jobs: usize) -> ReloadFeatureFactory<SingleDimLoad> {
    ReloadFeatureFactory::<SingleDimLoad>::new("shared_reload")
        .set_capacity_code(VIOLATION_CODE)
        .set_resource_code(VIOLATION_CODE)
        .set_belongs_to_route(belongs_to_route)
        .set_is_reload_single(is_reload_single)
        .set_load_schedule_threshold(|capacity| *capacity * 0.9)
        .set_shared_resource_capacity(|activity| {
            activity.job.as_ref().and_then(|job| {
                job.dimens
                    .get_vehicle_capacity()
                    .cloned()
                    .zip(job.dimens.get_value::<ResourceIdDimenKey, SharedResourceId>().cloned())
            })
        })
        .set_shared_demand_capacity(|single| single.dimens.get_job_demand().map(|demand| demand.delivery.0))
        .set_is_partial_solution(move |solution_ctx| solution_ctx.get_jobs_amount() != total_jobs)
}

fn create_shared_reload_feature(total_jobs: usize) -> Feature {
    create_shared_reload_builder(total_jobs).build_shared().expect("cannot create shared reload feature")
}

fn create_route_ctx(
    fleet: &Fleet,
    vehicle_id: &str,
    resources: &HashMap<usize, i32>,
    activities: &[ActivityType],
) -> RouteContext {
    let activities = activities.iter().map(|activity_type| match activity_type {
        SharedResource(resource_id) => {
            create_resource_activity(vehicle_id, *resources.get(resource_id).unwrap(), Some(*resource_id))
        }
        NormalResource(capacity) => create_resource_activity(vehicle_id, *capacity, None),
        Usage(demand) => create_usage_activity(*demand),
    });

    let mut route_ctx = RouteContextBuilder::default()
        .with_route(RouteBuilder::default().with_vehicle(fleet, vehicle_id).add_activities(activities).build())
        .build();

    let intervals = get_route_intervals(route_ctx.route(), {
        move |activity| activity.job.as_ref().map_or(false, |job| is_reload_single(job))
    });
    route_ctx.state_mut().set_reload_intervals(intervals);

    route_ctx
}

fn create_solution_ctx(
    resources: Vec<(usize, i32)>,
    activities: Vec<Vec<ActivityType>>,
    capacity: i32,
    is_ovrp: bool,
) -> SolutionContext {
    let resources = resources.into_iter().collect::<HashMap<usize, _>>();
    let (mut v1, mut v2) = if is_ovrp {
        (test_ovrp_vehicle("v1"), test_ovrp_vehicle("v2"))
    } else {
        (test_vehicle_with_id("v1"), test_vehicle_with_id("v2"))
    };
    v1.dimens.set_vehicle_capacity(SingleDimLoad::new(capacity));
    v2.dimens.set_vehicle_capacity(SingleDimLoad::new(capacity));

    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(v1).add_vehicle(v2).build();

    let routes = activities
        .into_iter()
        .enumerate()
        .map(|(idx, activities)| create_route_ctx(&fleet, format!("v{}", idx + 1).as_str(), &resources, &activities))
        .collect();

    TestInsertionContextBuilder::default().with_routes(routes).build().solution
}

enum ActivityType {
    Usage(i32),
    NormalResource(i32),
    SharedResource(usize),
}

parameterized_test! {can_update_resource_consumption, (resources, activities, total_jobs, expected_resources), {
    can_update_resource_consumption_impl(resources, activities, total_jobs, expected_resources);
}}

can_update_resource_consumption! {
    case_01_single_shared_resource: (vec![(0, 10)],
        vec![vec![Usage(2), SharedResource(0), Usage(2)], vec![Usage(2), SharedResource(0), Usage(2)]],
        None,
        vec![vec![None, None, Some(6), None, None], vec![None, None, Some(6), None, None]],
    ),

    case_02_two_shared_resources: (vec![(0, 10), (1, 10)],
        vec![vec![Usage(2), SharedResource(0), Usage(2)], vec![Usage(2), SharedResource(1), Usage(1)]],
        None,
        vec![vec![None, None, Some(8), None, None], vec![None, None, Some(9), None, None]],
    ),

    case_03_mixed_normal_resource: (vec![(0, 10), (1, 5)],
        vec![vec![Usage(2), SharedResource(0), Usage(2)], vec![Usage(2), NormalResource(10), Usage(2)]],
        None,
        vec![vec![None, None, Some(8), None, None], vec![None, None, None, None, None]],
    ),

    case_04_partial_solution: (vec![(0, 10)],
        vec![vec![Usage(2), SharedResource(0), Usage(2)], vec![Usage(2), SharedResource(0), Usage(2)]],
        Some(100),
        vec![vec![None, None, None, None, None], vec![None, None, None, None, None]],
    ),
}

fn can_update_resource_consumption_impl(
    resources: Vec<(usize, i32)>,
    activities: Vec<Vec<ActivityType>>,
    total_jobs: Option<usize>,
    expected_resources: Vec<Vec<Option<i32>>>,
) {
    let total_jobs = total_jobs.unwrap_or(activities[0].len() + activities[1].len());
    let mut solution_ctx = create_solution_ctx(resources, activities, 2, false);
    let state = create_shared_reload_feature(total_jobs).state.unwrap();

    state.accept_solution_state(&mut solution_ctx);

    let actual_resources = solution_ctx
        .routes
        .iter()
        .map(|route_ctx| {
            (0..route_ctx.route().tour.total())
                .map(|activity_idx| {
                    route_ctx
                        .state()
                        .get_activity_state::<SharedResourceStateKey, Option<SingleDimLoad>>(activity_idx)
                        .and_then(|resource| *resource)
                        .map(|resource| resource.value)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(actual_resources, expected_resources);
}

parameterized_test! {can_constraint_route, (resources, activities, total_jobs, job_demand, expected), {
    can_constraint_route_impl(resources, activities, total_jobs, job_demand, expected);
}}

can_constraint_route! {
    case_01_partial_solution: (vec![(0, 10)],
        vec![Usage(2), SharedResource(0), Usage(2)], 1, Some(1), Some(VIOLATION_CODE),
    ),

    case_02_complete_solution: (vec![(0, 10)],
        vec![Usage(2), SharedResource(0), Usage(2)], 3, Some(1), None,
    ),

    case_03_no_demand: (vec![(0, 10)],
        vec![Usage(2), SharedResource(0), Usage(2)], 3, None, None,
    ),

    case_04_no_resource: (vec![(0, 10)],
        vec![Usage(2), Usage(2)], 2, Some(1), None,
    ),
}

fn can_constraint_route_impl(
    resources: Vec<(usize, i32)>,
    activities: Vec<ActivityType>,
    total_jobs: usize,
    job_demand: Option<i32>,
    expected: Option<ViolationCode>,
) {
    let job = Job::Single(job_demand.map_or_else(
        || TestSingleBuilder::default().id("job1").build_shared(),
        |demand| TestSingleBuilder::default().demand(create_simple_demand(-demand)).build_shared(),
    ));
    // NOTE can use feature but test was written initially without full setup
    let builder = create_shared_reload_builder(total_jobs);
    let solution_ctx = create_solution_ctx(resources, vec![activities], 1, false);
    let constraint = SharedResourceConstraint {
        violation_code: VIOLATION_CODE,
        resource_demand_fn: builder.shared_resource_demand_fn.unwrap(),
        is_partial_solution_fn: builder.is_partial_solution_fn.unwrap(),
    };

    let result = constraint.evaluate(&MoveContext::route(&solution_ctx, &solution_ctx.routes[0], &job));

    assert_eq!(result.map(|result| result.code), expected);
}

parameterized_test! {can_constraint_activity, (resources, activities, insertion_idx, is_ovrp, job_demand, expected), {
    can_constraint_activity_impl(resources, activities, insertion_idx, is_ovrp, job_demand, expected);
}}

can_constraint_activity! {
    case_01_enough_resource: (vec![(0, 10)],
        vec![vec![Usage(2), SharedResource(0), Usage(2), Usage(2)], vec![SharedResource(0), Usage(6)]],
        0, false, Some(1), None,
    ),
    case_02_enough_resource: (vec![(0, 10), (1, 10)],
        vec![vec![SharedResource(0), Usage(2), Usage(2)], vec![SharedResource(1), Usage(6)]],
        2, false, Some(1), None,
    ),

    case_03_not_enough_resource: (vec![(0, 10)],
        vec![vec![SharedResource(0), Usage(2), Usage(2)], vec![SharedResource(0), Usage(6)]],
        2, false, Some(1), Some(VIOLATION_CODE),
    ),
    case_04_not_enough_resource: (vec![(0, 10)],
        vec![vec![SharedResource(0), Usage(2), Usage(2)], vec![SharedResource(0), Usage(5)]],
        2, false, Some(2), Some(VIOLATION_CODE),
    ),

    case_05_enough_resource_ovrp: (vec![(0, 1)],
        vec![vec![Usage(1), SharedResource(0), Usage(1)], vec![Usage(6)]],
        1, true, Some(1), None,
    ),
    case_06_not_enough_resource_ovrp: (vec![(0, 1)],
        vec![vec![Usage(1), SharedResource(0), Usage(1)], vec![Usage(6)]],
        2, true, Some(1), Some(VIOLATION_CODE),
    ),
    case_07_not_enough_resource_ovrp: (vec![(0, 1)],
        vec![vec![Usage(1), SharedResource(0), Usage(1)], vec![Usage(6)]],
        3, true, Some(1), Some(VIOLATION_CODE),
    ),

    case_08_not_enough_resource_ovrp: (vec![(0, 1)],
        vec![vec![SharedResource(0)], vec![Usage(1), SharedResource(0), Usage(1)]],
        0, true, Some(1), None,
    ),
    case_09_not_enough_resource_ovrp: (vec![(0, 1)],
        vec![vec![SharedResource(0)], vec![Usage(1), SharedResource(0), Usage(1)]],
        1, true, Some(1), Some(VIOLATION_CODE),
    ),
}

fn can_constraint_activity_impl(
    resources: Vec<(usize, i32)>,
    activities: Vec<Vec<ActivityType>>,
    insertion_idx: usize,
    is_ovrp: bool,
    demand: Option<i32>,
    expected: Option<ViolationCode>,
) {
    let target = demand.map_or_else(|| ActivityBuilder::default().build(), create_usage_activity);
    let total_jobs = activities[0].len() + activities[1].len();
    let mut solution_ctx = create_solution_ctx(resources, activities, 2, is_ovrp);
    // NOTE: feature combinator will merge shared resource with capacity constraint which requires more
    // complicated setup. The test was written initially without it, so keep it as it is.
    let builder = create_shared_reload_builder(total_jobs);
    let constraint = SharedResourceConstraint {
        violation_code: VIOLATION_CODE,
        resource_demand_fn: builder.shared_resource_demand_fn.as_ref().cloned().unwrap(),
        is_partial_solution_fn: builder.is_partial_solution_fn.as_ref().cloned().unwrap(),
    };
    let state = SharedResourceState {
        resource_capacity_fn: builder.shared_resource_capacity_fn.unwrap(),
        resource_demand_fn: builder.shared_resource_demand_fn.unwrap(),
        is_partial_solution_fn: builder.is_partial_solution_fn.unwrap(),
    };
    state.accept_solution_state(&mut solution_ctx);

    let activity_ctx =
        ActivityContext { index: insertion_idx, prev: &create_usage_activity(0), target: &target, next: None };

    let result = constraint.evaluate(&MoveContext::activity(&solution_ctx.routes[0], &activity_ctx));

    assert_eq!(result.map(|result| result.code), expected)
}
