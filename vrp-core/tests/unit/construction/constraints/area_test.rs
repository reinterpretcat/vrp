use super::*;
use crate::helpers::construction::constraints::create_constraint_pipeline_with_module;
use crate::helpers::models::domain::create_empty_solution_context;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::{Cost, ValueDimension};
use crate::models::problem::Fleet;

fn create_fleet(areas: Vec<Area>) -> Fleet {
    let mut vehicle = test_vehicle_with_id("v1");
    vehicle.dimens.set_value("areas", areas);

    FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(vehicle)
        .add_vehicle(test_vehicle_with_id("v2"))
        .build()
}

fn create_area_constraint_pipeline() -> ConstraintPipeline {
    create_constraint_pipeline_with_module(Arc::new(AreaModule::new(
        Arc::new(move |actor| actor.vehicle.dimens.get_value::<Vec<Area>>("areas")),
        Arc::new(|location| (location as f64, 0.)),
        2,
    )))
}

parameterized_test! {can_check_single_job, (vehicle_id, job_locations, activity_location, expected), {
    can_check_single_job_impl(vehicle_id, job_locations, activity_location, expected);
}}

can_check_single_job! {
    case01: ("v1", vec![Some(0)], 0, (None, None)),
    case02: ("v1", vec![Some(10)], 10, (Some(()), Some(()))),
    case03: ("v1", vec![Some(10), Some(0)], 10, (None, Some(()))),
    case04: ("v1", vec![Some(10), Some(0)], 0, (None, None)),
    case05: ("v1", vec![Some(10), Some(20)], 20, (Some(()), Some(()))),

    case06: ("v2", vec![Some(0)], 0, (None, None)),
    case07: ("v2", vec![Some(10)], 10, (None, None)),
}

fn can_check_single_job_impl(
    vehicle_id: &str,
    job_locations: Vec<Option<Location>>,
    activity_location: Location,
    expected: (Option<()>, Option<()>),
) {
    let areas = vec![Area { priority: None, outer_shape: vec![(-5., -5.), (-5., 5.), (5., 5.), (5., -5.)] }];
    let solution_ctx = create_empty_solution_context();
    let route_ctx = create_route_context_with_activities(&create_fleet(areas), vehicle_id, vec![]);
    let activity_ctx = ActivityContext {
        index: 0,
        prev: &test_activity_without_job(),
        target: &test_activity_with_location(activity_location),
        next: None,
    };
    let pipeline = create_area_constraint_pipeline();

    let route_result = pipeline.evaluate_hard_route(
        &solution_ctx,
        &route_ctx,
        &SingleBuilder::default()
            .places(job_locations.into_iter().map(|l| (l, 10., vec![(0., 100.)])).collect())
            .build_as_job_ref(),
    );
    let activity_result = pipeline.evaluate_hard_activity(&route_ctx, &activity_ctx);

    assert_eq!(route_result.map(|_| ()), expected.0);
    assert_eq!(activity_result.map(|_| ()), expected.1);
}

parameterized_test! {can_check_multi_job, (job_locations, expected), {
    can_check_multi_job_impl(job_locations, expected);
}}

can_check_multi_job! {
    case01: (vec![Some(0), Some(1)], None),
    case02: (vec![Some(0), Some(6)], Some(())),
    case03: (vec![Some(6), Some(0)], Some(())),
    case04: (vec![Some(10), Some(20)], Some(())),
}

fn can_check_multi_job_impl(job_locations: Vec<Option<Location>>, expected: Option<()>) {
    let areas = vec![Area { priority: None, outer_shape: vec![(-5., -5.), (-5., 5.), (5., 5.), (5., -5.)] }];
    let solution_ctx = create_empty_solution_context();
    let route_ctx = create_route_context_with_activities(&create_fleet(areas), "v1", vec![]);
    let pipeline = create_area_constraint_pipeline();
    let mut builder = MultiBuilder::default();
    job_locations.into_iter().for_each(|location| {
        builder.job(Arc::try_unwrap(test_single_with_location(location)).ok().unwrap());
    });

    let route_result = pipeline.evaluate_hard_route(&solution_ctx, &route_ctx, &builder.build());

    assert_eq!(route_result.map(|_| ()), expected);
}

#[test]
fn can_check_location_in_area() {
    let polygon = vec![(-1., -1.), (-1., 1.), (1., 1.), (1., -1.)];
    assert_eq!(is_location_in_area(&(0., 0.), &polygon), true);
    assert_eq!(is_location_in_area(&(2., 0.), &polygon), false);

    let polygon = vec![(1., 3.), (2., 8.), (5., 4.), (5., 9.), (7., 5.), (13., 1.), (3., 1.)];
    assert_eq!(is_location_in_area(&(5.5, 7.), &polygon), true);
    assert_eq!(is_location_in_area(&(4.5, 7.), &polygon), false);

    let polygon = vec![
        (52.499148, 13.485196),
        (52.498600, 13.480000),
        (52.503800, 13.474680),
        (52.510000, 13.468270),
        (52.510788, 13.466904),
        (52.512116, 13.465350),
        (52.512000, 13.467000),
        (52.513579, 13.471027),
        (52.512938, 13.472668),
        (52.511829, 13.474922),
        (52.507945, 13.480124),
        (52.509082, 13.482892),
        (52.536026, 13.490519),
        (52.534470, 13.499703),
        (52.499148, 13.485196),
    ];
    assert_eq!(is_location_in_area(&(52.508956, 13.483328), &polygon), true);
    assert_eq!(is_location_in_area(&(52.505, 13.48), &polygon), true);

    let polygon =
        vec![(52.481171, 13.4107070), (52.480248, 13.4101200), (52.480237, 13.4062790), (52.481161, 13.4062610)];
    assert_eq!(is_location_in_area(&(52.480890, 13.4081030), &polygon), true);
}

parameterized_test! {can_estimate_activity_with_penalty, (priority, route_cost, expected), {
    can_estimate_activity_with_penalty_impl(priority, route_cost, expected);
}}

can_estimate_activity_with_penalty! {
    case01: (None, None, 0.),
    case02: (Some(1), None, 0.),
    case03: (Some(2), None, 1E9),
    case04: (Some(3), None, 2E9),

    case05: (None, Some(100.), 0.),
    case06: (Some(1), Some(100.), 0.),
    case07: (Some(2), Some(100.), 400.),

    case08: (None, Some(0.), 0.),
    case09: (Some(1), Some(0.), 0.),
    case10: (Some(2), Some(0.), 1E9),
    case11: (Some(3), Some(0.), 2E9),
}

fn can_estimate_activity_with_penalty_impl(priority: Option<usize>, route_cost: Option<Cost>, expected_cost: Cost) {
    let areas = vec![Area { priority, outer_shape: vec![(-1., -1.), (-1., 1.), (1., 1.), (1., -1.)] }];
    let area_constraint = AreaSoftActivityConstraint {
        area_resolver: Arc::new(move |actor| actor.vehicle.dimens.get_value::<Vec<Area>>("areas")),
        location_resolver: Arc::new(|location| (location as f64, 0.)),
    };

    let mut route_ctx = create_route_context_with_activities(&create_fleet(areas), "v1", vec![]);

    if let Some(route_cost) = route_cost {
        route_ctx.state_mut().put_route_state(TOTAL_DISTANCE_KEY, route_cost);
    }

    let cost = area_constraint.estimate_activity(
        &route_ctx,
        &ActivityContext {
            index: 0,
            prev: &test_activity_with_location(1),
            target: &test_activity_with_location(0),
            next: None,
        },
    );

    assert_eq!(cost, expected_cost);
}
