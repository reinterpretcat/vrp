use super::*;
use crate::helpers::construction::features::single_demand_as_multi;
use crate::helpers::construction::heuristics::InsertionContextBuilder;
use crate::helpers::models::problem::{test_driver, FleetBuilder, SingleBuilder, VehicleBuilder};
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder};
use crate::models::solution::Activity;
use std::marker::PhantomData;

const VIOLATION_CODE: ViolationCode = 1;

#[derive(Clone)]
struct TestReloadAspects<T: LoadOps> {
    capacity_keys: CapacityKeys,
    phantom: PhantomData<T>,
}

impl<T: LoadOps> ReloadAspects<T> for TestReloadAspects<T> {
    fn belongs_to_route(&self, route: &Route, job: &Job) -> bool {
        job.as_single()
            .filter(|single| self.is_reload_single(single.as_ref()))
            .and_then(|single| single.dimens.get_value::<String>("vehicle_id"))
            .zip(route.actor.vehicle.dimens.get_id())
            .map_or(false, |(a, b)| a == b)
    }

    fn is_reload_single(&self, single: &Single) -> bool {
        single.dimens.get_value::<String>("type").map_or(false, |job_type| job_type == "reload")
    }

    fn get_capacity<'a>(&self, vehicle: &'a Vehicle) -> Option<&'a T> {
        vehicle.dimens.get_capacity()
    }

    fn get_demand<'a>(&self, single: &'a Single) -> Option<&'a Demand<T>> {
        single.dimens.get_demand()
    }
}

impl<T: LoadOps> CapacityAspects<T> for TestReloadAspects<T> {
    fn get_capacity<'a>(&self, vehicle: &'a Vehicle) -> Option<&'a T> {
        vehicle.dimens.get_capacity()
    }

    fn get_demand<'a>(&self, single: &'a Single) -> Option<&'a Demand<T>> {
        single.dimens.get_demand()
    }

    fn set_demand(&self, single: &mut Single, demand: Demand<T>) {
        single.dimens.set_demand(demand);
    }

    fn get_state_keys(&self) -> &CapacityKeys {
        &self.capacity_keys
    }

    fn get_violation_code(&self) -> ViolationCode {
        VIOLATION_CODE
    }
}

fn create_activity_with_demand(
    job_id: &str,
    pickup: (i32, i32),
    delivery: (i32, i32),
    activity_type: &str,
) -> Activity {
    ActivityBuilder::default()
        .job(Some(
            SingleBuilder::default()
                .id(job_id)
                .demand(single_demand_as_multi(pickup, delivery))
                .property("type", activity_type.to_string())
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
            SingleBuilder::default()
                .id(reload_id)
                .property("type", "reload".to_string())
                .property("vehicle_id", "v1".to_string())
                .build_shared(),
        ))
        .build()
}

fn create_route_context(capacity: Vec<i32>, activities: Vec<Activity>) -> RouteContext {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(VehicleBuilder::default().id("v1").capacity_mult(capacity).build())
        .build();

    RouteContextBuilder::default()
        .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").add_activities(activities).build())
        .build()
}

fn create_reload_keys() -> ReloadKeys {
    let mut state_registry = StateKeyRegistry::default();
    ReloadKeys { intervals: state_registry.next_key(), capacity_keys: CapacityKeys::from(&mut state_registry) }
}

#[test]
fn can_handle_reload_jobs_with_merge() {
    let create_reload_job = || Job::Single(reload("reload").job.unwrap());
    let create_job = || SingleBuilder::default().location(None).build_as_job_ref();
    let reload_keys = create_reload_keys();
    let feature = create_simple_reload_multi_trip_feature(
        "reload",
        {
            let reload_keys = reload_keys.clone();
            Box::new(move |name, route_intervals| {
                create_capacity_limit_with_multi_trip_feature::<SingleDimLoad, _>(
                    name,
                    route_intervals,
                    TestReloadAspects { capacity_keys: reload_keys.capacity_keys, phantom: Default::default() },
                )
            })
        },
        Box::new(|_| SingleDimLoad::default()),
        reload_keys.clone(),
        TestReloadAspects { capacity_keys: reload_keys.capacity_keys, phantom: Default::default() },
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

    let reload_keys = create_reload_keys();
    let mut solution_ctx = InsertionContextBuilder::default()
        .with_routes(vec![create_route_context(vec![capacity], activities)])
        .build()
        .solution;
    let reload_feature = create_simple_reload_multi_trip_feature::<MultiDimLoad, _>(
        "reload",
        Box::new({
            let capacity_keys = reload_keys.capacity_keys.clone();
            move |name, route_intervals| {
                create_capacity_limit_with_multi_trip_feature::<MultiDimLoad, _>(
                    name,
                    route_intervals,
                    TestReloadAspects { capacity_keys, phantom: Default::default() },
                )
            }
        }),
        Box::new(move |capacity| *capacity * threshold),
        reload_keys.clone(),
        TestReloadAspects { capacity_keys: reload_keys.capacity_keys, phantom: Default::default() },
    )
    .unwrap();
    let min_jobs_feature = create_minimize_unassigned_jobs_feature("min_jobs", Arc::new(|_, _| 1.)).unwrap();
    let goal = GoalContextBuilder::with_features(vec![reload_feature, min_jobs_feature])
        .unwrap()
        .set_goal(&["min_jobs"], &["min_jobs"])
        .unwrap()
        .build()
        .unwrap();

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
            .filter_map(|job| job.dimens.get_id())
            .collect::<Vec<_>>(),
        expected
    );
}

parameterized_test! {can_handle_new_interval_needed_for_multi_dim_load, (vehicle_capacity, current_capacity, expected), {
    can_handle_new_interval_needed_for_multi_dim_load_impl(vehicle_capacity, current_capacity, expected);
}}

can_handle_new_interval_needed_for_multi_dim_load! {
    case01_all_the_same: (vec![1], vec![1], true),
    case02_one_the_same: (vec![2, 1], vec![1, 1], true),
    case03_all_different: (vec![2, 2], vec![1, 1], false),
}

fn can_handle_new_interval_needed_for_multi_dim_load_impl(
    vehicle_capacity: Vec<i32>,
    current_capacity: Vec<i32>,
    expected: bool,
) {
    let threshold = 1.;
    let reload_keys = create_reload_keys();
    let route_intervals = create_reload_route_intervals::<MultiDimLoad, _>(
        reload_keys.clone(),
        Box::new(move |capacity| *capacity * threshold),
        None,
        TestReloadAspects { capacity_keys: reload_keys.capacity_keys.clone(), phantom: Default::default() },
    );
    let mut route_ctx = create_route_context(vehicle_capacity, Vec::default());
    let (route, state) = route_ctx.as_mut();
    let mut current_capacities = vec![MultiDimLoad::default(); route.tour.total()];
    current_capacities[route.tour.end_idx().unwrap()] = MultiDimLoad::new(current_capacity);
    state.put_activity_states(reload_keys.capacity_keys.max_past_capacity, current_capacities);

    let result = route_intervals.is_new_interval_needed(&route_ctx);

    assert_eq!(result, expected);
}
