use super::*;
use crate::helpers::*;
use crate::helpers::{create_single_with_location, get_costs};
use vrp_core::construction::heuristics::*;
use vrp_core::models::common::{MultiDimLoad, SingleDimLoad};
use vrp_core::models::problem::*;

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
    single_mut.dimens.set_value("shift_index", 0);
    single_mut.dimens.set_value("vehicle_id", "v1");

    Activity { job: Some(single_shared), ..create_activity_at_location(0) }
}

#[test]
fn can_handle_reload_jobs_with_merge() {
    let create_reload_job = || Job::Single(reload("reload").job.unwrap());
    let create_job = || Job::Single(Arc::new(create_single_with_location(None)));
    let (transport, activity) = get_costs();
    let multi_trip = Arc::new(ReloadMultiTrip::new(activity, transport, Box::new(|_| SingleDimLoad::default())));
    let constraint = CapacityConstraintModule::<SingleDimLoad>::new_with_multi_trip(2, multi_trip);

    assert_eq!(constraint.merge(create_reload_job(), create_job()).map(|_| ()), Err(2));
    assert_eq!(constraint.merge(create_job(), create_reload_job()).map(|_| ()), Err(2));
    assert_eq!(constraint.merge(create_reload_job(), create_reload_job()).map(|_| ()), Err(2));
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

    case06_keep_static_capacity: (
        vec![delivery("d1", (1, 0)), delivery("d2", (1, 0)), reload("r1"), delivery("d3", (1, 0))],
        2, vec!["d1", "d2", "r1", "d3"]
    ),
    case07_keep_static_capacity: (
        vec![delivery("d1", (2, 0)), reload("r1"), delivery("d2", (1, 0))],
        2, vec!["d1", "r1", "d2"]
    ),
    case08_keep_static_capacity: (
        vec![delivery("d1", (2, 0)), reload("r1"), delivery("d2", (2, 0))],
        3, vec!["d1", "r1", "d2"]
    ),
}

fn can_remove_trivial_reloads_when_used_from_capacity_constraint_impl(
    activities: Vec<Activity>,
    capacity: i32,
    expected: Vec<&str>,
) {
    let threshold = 0.9;
    let mut vehicle = test_vehicle("v1");
    vehicle.dimens.set_capacity(MultiDimLoad::new(vec![capacity]));
    let fleet = test_fleet_with_vehicles(vec![Arc::new(vehicle)]);
    let route_ctx = RouteContext::new_with_state(
        Arc::new(create_route_with_activities(&fleet, "v1", activities)),
        Arc::new(RouteState::default()),
    );
    let mut solution_ctx = SolutionContext { routes: vec![route_ctx], ..create_solution_context_for_fleet(&fleet) };
    let (transport, activity) = get_costs();
    let reload = Arc::new(ReloadMultiTrip::<MultiDimLoad>::new(
        activity,
        transport,
        Box::new(move |capacity| *capacity * threshold),
    ));
    let constraint = CapacityConstraintModule::new_with_multi_trip(1, reload);
    constraint.accept_route_state(&mut solution_ctx.routes.get_mut(0).unwrap());

    constraint.accept_solution_state(&mut solution_ctx);

    assert_eq!(solution_ctx.routes.get(0).unwrap().route.tour
        .all_activities().filter_map(|activity| activity.job.as_ref())
        .filter_map(|job| job.dimens.get_id().map(|id|id.as_str()))
        .collect::<Vec<_>>(), expected);

}
