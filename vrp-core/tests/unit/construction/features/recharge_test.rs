use super::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder};
use crate::models::solution::Activity;

const VIOLATION_CODE: ViolationCode = 1;

struct VehicleIdDimenKey;
struct JobTypeDimenKey;

fn create_recharge_feature(limit: Distance) -> Feature {
    fn is_recharge_single(single: &Single) -> bool {
        single.dimens.get_value::<JobTypeDimenKey, String>().map_or(false, |job_type| job_type == "recharge")
    }

    RechargeFeatureBuilder::new("recharge")
        .set_transport(TestTransportCost::new_shared())
        .set_violation_code(VIOLATION_CODE)
        .set_distance_limit(move |_: &Actor| Some(limit))
        .set_is_recharge_single(is_recharge_single)
        .set_belongs_to_route(|route, job| {
            job.as_single()
                .filter(|single| is_recharge_single(single))
                .and_then(|single| single.dimens.get_value::<VehicleIdDimenKey, String>())
                .zip(route.actor.vehicle.dimens.get_vehicle_id())
                .map_or(false, |(a, b)| a == b)
        })
        .build()
        .unwrap()
}

fn recharge(location: Location) -> Activity {
    ActivityBuilder::with_location(location)
        .job(Some(
            TestSingleBuilder::default()
                .id("recharge")
                .property::<JobTypeDimenKey, _>("recharge".to_string())
                .property::<VehicleIdDimenKey, _>("v1".to_string())
                .build_shared(),
        ))
        .build()
}

fn create_route_ctx(activities: &[Location], recharges: Vec<(usize, Location)>, is_open_end: bool) -> RouteContext {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(if is_open_end { test_ovrp_vehicle("v1") } else { test_vehicle_with_id("v1") })
        .build();

    let mut route_ctx = RouteContextBuilder::default()
        .with_route(
            RouteBuilder::default()
                .with_vehicle(&fleet, "v1")
                .add_activities(activities.iter().enumerate().map(|(idx, &location)| {
                    ActivityBuilder::with_location(location)
                        .schedule(Schedule::new(location as f64, location as f64))
                        .job(Some(TestSingleBuilder::default().id(&format!("job{}", idx + 1)).build_shared()))
                        .build()
                }))
                .build(),
        )
        .build();

    recharges.into_iter().for_each(|(recharge_idx, recharge_location)| {
        route_ctx.route_mut().tour.insert_at(recharge(recharge_location), recharge_idx);
    });

    route_ctx
}

parameterized_test! {can_accumulate_distance, (limit, recharges, activities, expected_counters), {
    can_accumulate_distance_impl(limit, recharges, activities, expected_counters);
}}

can_accumulate_distance! {
    case01_single_recharge: (20., vec![(2, 8)],
        vec![5, 10, 15], vec![0., 5., 8., 2., 7.]
    ),
    case02_two_recharges: (20., vec![(2, 8), (5, 17)],
        vec![5, 10, 15, 20], vec![0., 5., 8., 2., 7., 9., 3.]
    ),
    case03_no_recharges: (20., vec![],
        vec![5, 10, 15, 20], vec![0., 5., 10., 15., 20.]
    ),
    case04_recharge_at_end: (25., vec![(4, 8)],
        vec![5, 10, 15], vec![0., 5., 10., 15., 22.]
    ),
    case05_recharge_at_start: (20., vec![(1, 8)],
        vec![5, 10, 15], vec![0., 8., 3., 8., 13.]
    ),
}

fn can_accumulate_distance_impl(
    limit: Distance,
    recharges: Vec<(usize, Location)>,
    activities: Vec<Location>,
    expected_counters: Vec<Distance>,
) {
    let mut route_ctx = create_route_ctx(&activities, recharges, true);
    let feature = create_recharge_feature(limit);
    let state = feature.state.unwrap();

    state.accept_route_state(&mut route_ctx);

    (0..route_ctx.route().tour.total()).for_each(|activity_idx| {
        let counter = route_ctx.state().get_recharge_distance_at(activity_idx).copied().unwrap_or_default();
        assert_eq!(counter, expected_counters[activity_idx], "doesn't match for: {activity_idx}");
    });
}

parameterized_test! {can_evaluate_insertion, (limit, recharges, insertion_data, activities, expected), {
    can_evaluate_insertion_impl(limit, recharges, insertion_data, activities, expected);
}}

can_evaluate_insertion! {
    case01_reject_before_recharge: (20., vec![(2, 8)], (1, 16, (1, 2)), vec![5, 10, 15],
        ConstraintViolation::skip(VIOLATION_CODE),
    ),
    case02_accept_after_recharge: (20., vec![(2, 8)], (1, 16, (2, 3)), vec![5, 10, 15],
        None,
    ),
}

fn can_evaluate_insertion_impl(
    limit: Distance,
    recharges: Vec<(usize, Location)>,
    insertion_data: (usize, Location, (usize, usize)),
    activities: Vec<Location>,
    expected: Option<ConstraintViolation>,
) {
    let (index, new_location, (prev, next)) = insertion_data;
    let mut route_ctx = create_route_ctx(&activities, recharges, true);
    let feature = create_recharge_feature(limit);
    let (constraint, state) = (feature.constraint.unwrap(), feature.state.unwrap());
    state.accept_route_state(&mut route_ctx);

    let result = constraint.evaluate(&MoveContext::Activity {
        route_ctx: &route_ctx,
        activity_ctx: &ActivityContext {
            index,
            prev: route_ctx.route().tour.get(prev).unwrap(),
            target: &ActivityBuilder::with_location(new_location)
                .job(Some(TestSingleBuilder::default().build_shared()))
                .build(),
            next: route_ctx.route().tour.get(next),
        },
    });

    assert_eq!(result, expected);
}

parameterized_test! {can_handle_obsolete_intervals, (limit, recharges, activities, expected), {
    can_handle_obsolete_intervals_impl(limit, recharges, activities, expected);
}}

can_handle_obsolete_intervals! {
    case01_remove_one_exact: (30., vec![(2, 5)], vec![5, 10, 15, 20, 30], vec![0, 5, 10, 15, 20, 30]),
    case02_remove_one_diff: (30., vec![(3, 8)], vec![5, 10, 15, 20, 30], vec![0, 5, 10, 15, 20, 30]),
    case03_keep_one_exact: (29., vec![(2, 5)], vec![5, 10, 15, 20, 30], vec![0, 5, 5, 10, 15, 20, 30]),
    case04_remove_one_diff: (29., vec![(3, 8)], vec![5, 10, 15, 20, 30], vec![0, 5, 10, 8, 15, 20, 30]),

    case05_can_handle_two: (25., vec![(3, 10), (5, 20)], vec![5, 10, 15, 20, 30], vec![0, 5, 10, 15, 20, 20, 30]),
}

fn can_handle_obsolete_intervals_impl(
    limit: Distance,
    recharges: Vec<(usize, Location)>,
    activities: Vec<Location>,
    expected: Vec<Location>,
) {
    let mut solution = TestInsertionContextBuilder::default()
        .with_routes(vec![create_route_ctx(&activities, recharges, true)])
        .build()
        .solution;
    let feature = create_recharge_feature(limit);
    let state = feature.state.unwrap();

    state.accept_solution_state(&mut solution);

    assert_eq!(
        expected,
        solution.routes[0].route().tour.all_activities().map(|a| a.place.location).collect::<Vec<_>>()
    );
}

#[test]
fn can_accept_recharge_in_long_empty_route() {
    let mut route_ctx = create_route_ctx(&[], vec![], false);
    route_ctx.route_mut().tour.get_mut(1).unwrap().place.location = 100;
    let feature = create_recharge_feature(55.);
    let (constraint, state) = (feature.constraint.unwrap(), feature.state.unwrap());
    state.accept_route_state(&mut route_ctx);

    let result = constraint.evaluate(&MoveContext::Activity {
        route_ctx: &route_ctx,
        activity_ctx: &ActivityContext {
            index: 0,
            prev: route_ctx.route().tour.start().unwrap(),
            target: &recharge(50),
            next: route_ctx.route().tour.end(),
        },
    });

    assert_eq!(result, None);
}
