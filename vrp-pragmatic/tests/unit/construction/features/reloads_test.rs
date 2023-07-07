use super::*;
use crate::helpers::create_single_with_location;
use crate::helpers::*;
use vrp_core::construction::features::create_capacity_limit_with_multi_trip_feature;
use vrp_core::construction::heuristics::*;
use vrp_core::models::common::{MultiDimLoad, SingleDimLoad};
use vrp_core::models::problem::*;

const VIOLATION_CODE: ViolationCode = 1;

fn create_activity_with_demand(
    job_id: &str,
    pickup: (i32, i32),
    delivery: (i32, i32),
    activity_type: &str,
) -> Activity {
    let mut single_shared = create_single_with_type(job_id, activity_type);
    let single_mut = Arc::get_mut(&mut single_shared).unwrap();
    single_mut.dimens.set_demand(single_demand_as_multi(pickup, delivery));

    Activity { job: Some(single_shared), ..create_activity_at_location(0) }
}

fn pickup(job_id: &str, demand: (i32, i32)) -> Activity {
    create_activity_with_demand(job_id, demand, (0, 0), "pickup")
}

fn delivery(job_id: &str, demand: (i32, i32)) -> Activity {
    create_activity_with_demand(job_id, (0, 0), demand, "delivery")
}

fn reload(reload_id: &str) -> Activity {
    let mut single_shared = create_single_with_type(reload_id, "reload");
    let single_mut = Arc::get_mut(&mut single_shared).unwrap();
    single_mut.dimens.set_shift_index(0).set_vehicle_id("v1".to_string());

    Activity { job: Some(single_shared), ..create_activity_at_location(0) }
}

fn create_route_context_with_fleet(capacity: Vec<i32>, activities: Vec<Activity>) -> (RouteContext, Fleet) {
    let mut vehicle = test_vehicle("v1");
    vehicle.dimens.set_capacity(MultiDimLoad::new(capacity));
    let fleet = test_fleet_with_vehicles(vec![Arc::new(vehicle)]);
    let route_ctx =
        RouteContext::new_with_state(create_route_with_activities(&fleet, "v1", activities), RouteState::default());

    (route_ctx, fleet)
}

#[test]
fn can_handle_reload_jobs_with_merge() {
    let create_reload_job = || Job::Single(reload("reload").job.unwrap());
    let create_job = || Job::Single(Arc::new(create_single_with_location(None)));
    let feature = create_simple_reload_multi_trip_feature(
        "reload",
        Box::new(|name, multi_trip| create_capacity_limit_with_multi_trip_feature(name, VIOLATION_CODE, multi_trip)),
        Box::new(|_| SingleDimLoad::default()),
    );
    let constraint = feature.unwrap().constraint.unwrap();

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

    let (route_ctx, fleet) = create_route_context_with_fleet(vec![capacity], activities);
    let mut solution_ctx = SolutionContext { routes: vec![route_ctx], ..create_solution_context_for_fleet(&fleet) };
    let feature = create_simple_reload_multi_trip_feature::<MultiDimLoad>(
        "reload",
        Box::new(|name, multi_trip| create_capacity_limit_with_multi_trip_feature(name, VIOLATION_CODE, multi_trip)),
        Box::new(move |capacity| *capacity * threshold),
    )
    .unwrap();
    let variant = GoalContext::new(&[feature], &[], &[]).unwrap();

    variant.accept_route_state(solution_ctx.routes.get_mut(0).unwrap());

    variant.accept_solution_state(&mut solution_ctx);

    assert_eq!(
        solution_ctx
            .routes
            .get(0)
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

parameterized_test! {can_handle_multi_trip_needed_for_multi_dim_load, (vehicle_capacity, current_capacity, expected), {
    can_handle_multi_trip_needed_for_multi_dim_load_impl(vehicle_capacity, current_capacity, expected);
}}

can_handle_multi_trip_needed_for_multi_dim_load! {
    case01_all_the_same: (vec![1], vec![1], true),
    case02_one_the_same: (vec![2, 1], vec![1, 1], true),
    case03_all_different: (vec![2, 2], vec![1, 1], false),
}

fn can_handle_multi_trip_needed_for_multi_dim_load_impl(
    vehicle_capacity: Vec<i32>,
    current_capacity: Vec<i32>,
    expected: bool,
) {
    let threshold = 1.;
    let multi_trip = create_reload_multi_trip::<MultiDimLoad>(Box::new(move |capacity| *capacity * threshold), None);
    let (mut route_ctx, _) = create_route_context_with_fleet(vehicle_capacity, Vec::default());
    let (route, state) = route_ctx.as_mut();
    state.put_activity_state(MAX_PAST_CAPACITY_KEY, route.tour.end().unwrap(), MultiDimLoad::new(current_capacity));

    let result = multi_trip.is_multi_trip_needed(&route_ctx);

    assert_eq!(result, expected);
}
