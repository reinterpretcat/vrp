use crate::construction::enablers::{TotalDistanceTourState, TotalDurationTourState};
use crate::construction::features::MaxVehicleLoadTourState;
use crate::construction::heuristics::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder, RouteStateBuilder};
use rosomaxa::prelude::Float;

#[test]
fn can_extract_rosomaxa_features() {
    let mut insertion_ctx = TestInsertionContextBuilder::default().build();
    insertion_ctx.solution.routes.extend((0..4).map(|idx| {
        let activities =
            (0..=idx).map(|activity_idx| ActivityBuilder::with_location(idx * 10 + activity_idx + 1).build());
        let route = RouteBuilder::default().add_activities(activities).build();
        let state = RouteStateBuilder::default()
            .set_route_state(|state| {
                state.set_max_vehicle_load(idx as Float / 3.);
                state.set_total_duration((idx * 3) as Float);
                state.set_total_distance((idx * 5) as Float);
            })
            .build();

        RouteContextBuilder::default().with_route(route).with_state(state).build()
    }));

    let actual = Vec::from(get_rosomaxa_solution_features(&insertion_ctx));
    let expected = [
        0.1388888888888889,
        0.5,
        0.25,
        4.5,
        0.,
        7.5,
        17.5,
        16.,
        17.5,
        12.666666666666668,
        17.5,
        1.118033988749895,
        insertion_ctx.solution.unassigned.len() as Float,
        4.,
        insertion_ctx.get_total_cost().unwrap_or_default(),
    ];

    assert_eq!(actual.len(), expected.len());
    actual.iter().zip(expected).enumerate().for_each(|(idx, (actual, expected))| {
        assert!((actual - expected).abs() < 1E-12, "unexpected feature at index {idx}: {actual} != {expected}");
    });
}

#[test]
fn can_extract_features_without_routes() {
    let insertion_ctx = TestInsertionContextBuilder::default().build();
    let actual = Vec::from(get_rosomaxa_solution_features(&insertion_ctx));
    let expected = [vec![0.; 12], vec![insertion_ctx.solution.unassigned.len() as Float, 0., 0.]].concat();

    assert_eq!(actual, expected);
}
