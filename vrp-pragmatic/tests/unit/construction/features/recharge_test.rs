use super::*;
use crate::helpers::*;
use vrp_core::models::solution::Activity;

const VIOLATION_CODE: ViolationCode = 1;

fn recharge(location: Location) -> Activity {
    let mut single_shared = create_single_with_type("recharge", "recharge");
    let single_mut = Arc::get_mut(&mut single_shared).unwrap();
    single_mut.dimens.set_shift_index(0).set_vehicle_id("v1".to_string());

    Activity { job: Some(single_shared), ..create_activity_at_location(location) }
}

fn create_route_ctx(activities: &[(Timestamp, Timestamp, Location)], is_open_end: bool) -> RouteContext {
    let fleet = if is_open_end {
        test_fleet_with_vehicles(vec![Arc::new(test_vehicle_with_no_end("v1"))])
    } else {
        test_fleet()
    };

    RouteContext::new_with_state(
        create_route_with_activities(
            &fleet,
            "v1",
            activities
                .iter()
                .enumerate()
                .map(|(idx, &(arrival, departure, location))| Activity {
                    schedule: Schedule::new(arrival, departure),
                    job: Some(create_single(&format!("job{}", idx + 1))),
                    ..create_activity_at_location(location)
                })
                .collect(),
        ),
        RouteState::default(),
    )
}

fn create_feature(limit: Distance) -> Feature {
    create_recharge_feature(
        "recharge",
        VIOLATION_CODE,
        Arc::new(move |_: &Actor| Some(limit)),
        TestTransportCost::new_shared(),
    )
    .expect("cannot create feature")
}

parameterized_test! {can_accumulate_distance, (limit, recharges, activities, expected_counters), {
    can_accumulate_distance_impl(limit, recharges, activities, expected_counters);
}}

can_accumulate_distance! {
    case01_single_recharge: (20., vec![(2, 8)],
        vec![(5., 5., 5), (10., 10., 10), (15., 15., 15)],
        vec![0., 5., 0., 2., 7.]
    ),
    case02_two_recharges: (20., vec![(2, 8), (5, 17)],
        vec![(5., 5., 5), (10., 10., 10), (15., 15., 15), (20., 20., 20)],
        vec![0., 5., 0., 2., 7., 0., 3.]
    ),
    case03_no_recharges: (20., vec![],
        vec![(5., 5., 5), (10., 10., 10), (15., 15., 15), (20., 20., 20)],
        vec![0., 5., 10., 15., 20.]
    ),
    case04_recharge_at_end: (20., vec![(4, 8)],
        vec![(5., 5., 5), (10., 10., 10), (15., 15., 15)],
        vec![0., 5., 10., 15., 0.]
    ),
    case05_recharge_at_start: (20., vec![(1, 8)],
        vec![(5., 5., 5), (10., 10., 10), (15., 15., 15)],
        vec![0., 0., 3., 8., 13.]
    ),
}

fn can_accumulate_distance_impl(
    limit: Distance,
    recharges: Vec<(usize, Location)>,
    activities: Vec<(Timestamp, Timestamp, Location)>,
    expected_counters: Vec<Distance>,
) {
    let mut route_ctx = create_route_ctx(&activities, true);
    recharges.into_iter().for_each(|(recharge_idx, recharge_location)| {
        route_ctx.route_mut().tour.insert_at(recharge(recharge_location), recharge_idx);
    });
    let state = create_feature(limit).state.unwrap();

    state.accept_route_state(&mut route_ctx);

    route_ctx.route().tour.all_activities().enumerate().for_each(|(idx, activity)| {
        let counter = route_ctx
            .state()
            .get_activity_state::<Distance>(RECHARGE_DISTANCE_KEY, activity)
            .copied()
            .unwrap_or_default();
        assert_eq!(counter, expected_counters[idx], "doesn't match for: {}", idx);
    });
}
