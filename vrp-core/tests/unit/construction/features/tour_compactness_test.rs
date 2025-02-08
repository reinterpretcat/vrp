use super::*;
use crate::helpers::models::problem::{test_driver, FleetBuilder};
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder};
use crate::prelude::{VehicleBuilder, VehicleDetailBuilder};

const TEST_ESTIMATE: Cost = 10.;

fn create_objective(
    num_representative_points: usize,
) -> TourCompactnessObjective<impl Fn(&Profile, Location, Location) -> Cost + Send + Sync + 'static> {
    struct TestObjective;
    impl FeatureObjective for TestObjective {
        fn fitness(&self, _: &InsertionContext) -> Cost {
            todo!()
        }

        fn estimate(&self, _: &MoveContext<'_>) -> Cost {
            TEST_ESTIMATE
        }
    }

    let distance_fn = |_: &Profile, from: Location, to: Location| -> Cost { (to as f64 - from as f64).abs() };

    TourCompactnessObjective { objective: Arc::new(TestObjective), num_representative_points, distance_fn }
}

fn create_test_route_with_load(locations: Vec<Location>, max_load: f64) -> RouteContext {
    let detail = VehicleDetailBuilder::default().set_start_location(0);
    let vehicle = VehicleBuilder::default().id("v1").add_detail(detail.build().unwrap()).build().unwrap();
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();

    let mut state = RouteState::default();
    state.set_max_vehicle_load(max_load);

    RouteContextBuilder::default()
        .with_route(
            RouteBuilder::default()
                .with_vehicle(&fleet, "v1")
                .add_activities(locations.into_iter().map(|loc| ActivityBuilder::with_location(loc).build()))
                .build(),
        )
        .with_state(state)
        .build()
}

fn round(value: f64) -> f64 {
    (value * 1000.).round() / 1000.
}

mod representative_activities {
    use super::*;

    fn create_test_route(size: usize, is_closed: bool) -> RouteContext {
        let detail = VehicleDetailBuilder::default().set_start_location(0);
        let detail = if is_closed { detail.set_end_location(0) } else { detail };
        let vehicle = VehicleBuilder::default().id("v1").add_detail(detail.build().unwrap()).build().unwrap();
        let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();

        RouteContextBuilder::default()
            .with_route(
                RouteBuilder::default()
                    .with_vehicle(&fleet, "v1")
                    .add_activities((1..=size).map(|i| ActivityBuilder::with_location(i).build()))
                    .build(),
            )
            .build()
    }

    fn get_activity_locations(
        route_ctx: &RouteContext,
        with_start: bool,
        with_end: bool,
        num_representative_points: usize,
    ) -> Vec<Location> {
        create_objective(num_representative_points)
            .get_representative_activities(route_ctx, with_start, with_end)
            .map(|a| a.place.location)
            .collect()
    }

    #[test]
    fn test_empty_open_tour() {
        let route_ctx = create_test_route(0, false);

        assert_eq!(get_activity_locations(&route_ctx, false, true, 3), vec![0]);
    }

    #[test]
    fn test_empty_closed_tour() {
        let route_ctx = create_test_route(0, true);

        assert_eq!(get_activity_locations(&route_ctx, false, false, 3), vec![0, 0]);
    }

    #[test]
    fn test_small_open_tour_without_start() {
        let route_ctx = create_test_route(2, false); // Activities: [0 (start), 1, 2]

        assert_eq!(get_activity_locations(&route_ctx, false, true, 3), vec![0, 1, 2]);
    }

    #[test]
    fn test_small_open_tour_without_start_end() {
        let route_ctx = create_test_route(1, false); // Activities: [0 (start), 1]

        assert_eq!(get_activity_locations(&route_ctx, false, false, 3), vec![0, 1]);
    }

    #[test]
    fn test_closed_tour_exclude_end() {
        let route_ctx = create_test_route(3, true); // Activities: [0,1,2,3,0]

        assert_eq!(get_activity_locations(&route_ctx, true, false, 3), vec![0, 2, 3]);
    }

    #[test]
    fn test_closed_tour_same_amount_as_representative_num() {
        let route_ctx = create_test_route(3, true); // Activities: [0,1,2,3,0]

        assert_eq!(get_activity_locations(&route_ctx, false, false, 3), vec![1, 2, 3]);
    }

    #[test]
    fn test_all_activities_included_when_total_less_than_representative_num() {
        let route_ctx = create_test_route(2, true); // Activities: [0,1,2,0]

        assert_eq!(get_activity_locations(&route_ctx, false, true, 5), vec![0, 1, 2, 0]);
    }

    #[test]
    fn test_open_tour_basic() {
        let is_closed = false;
        let route_ctx = create_test_route(5, is_closed);

        assert_eq!(get_activity_locations(&route_ctx, false, !is_closed, 3), vec![1, 3, 5]);
    }

    #[test]
    fn test_closed_tour_with_diff_start_and_same_end() {
        let route_ctx = create_test_route(5, true);

        assert_eq!(get_activity_locations(&route_ctx, true, false, 3), vec![0, 2, 5]);
    }

    #[test]
    fn test_closed_tour_with_same_start_and_same_end() {
        let route_ctx = create_test_route(5, true);

        assert_eq!(get_activity_locations(&route_ctx, false, false, 3), vec![1, 3, 5]);
    }

    #[test]
    fn test_closed_large_tour_with_start_and_without_end() {
        let route_ctx = create_test_route(6, true);

        assert_eq!(get_activity_locations(&route_ctx, true, false, 4), vec![0, 2, 4, 6]);
    }

    #[test]
    fn test_fractional_step_rounding() {
        let route_ctx = create_test_route(6, false);

        assert_eq!(get_activity_locations(&route_ctx, false, true, 3), vec![1, 3, 6]);
    }

    #[test]
    fn test_fractional_step_rounding_closed() {
        let route_ctx = create_test_route(6, true);

        assert_eq!(get_activity_locations(&route_ctx, false, false, 3), vec![1, 3, 6]);
    }
}

mod calculate_distance {
    use super::*;

    fn create_test_route_with_locations(
        start: Location,
        locations: Vec<Location>,
        end: Option<Location>,
    ) -> RouteContext {
        let detail = VehicleDetailBuilder::default().set_start_location(start);
        let detail = if let Some(end_loc) = end { detail.set_end_location(end_loc) } else { detail };
        let vehicle = VehicleBuilder::default().id("v1").add_detail(detail.build().unwrap()).build().unwrap();
        let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();

        RouteContextBuilder::default()
            .with_route(
                RouteBuilder::default()
                    .with_vehicle(&fleet, "v1")
                    .add_activities(locations.into_iter().map(|loc| ActivityBuilder::with_location(loc).build()))
                    .build(),
            )
            .build()
    }

    #[test]
    fn test_basic_distance_calculation() {
        let objective = create_objective(3);

        let this_route = create_test_route_with_locations(0, vec![], Some(4));
        let other_route = create_test_route_with_locations(0, vec![5, 6, 7], Some(8));

        let distance = objective.calculate_distance(&this_route, &other_route, 2);
        assert_eq!(distance, 3.); // Closest activity is 5 (|2 - 5| = 3)
    }

    #[test]
    fn test_same_start_and_end_locations() {
        let objective = create_objective(3);

        let this_route = create_test_route_with_locations(0, vec![], Some(4));
        let other_route = create_test_route_with_locations(0, vec![1, 2, 3], Some(4));

        let distance = objective.calculate_distance(&this_route, &other_route, 2);
        assert_eq!(distance, 0.); // Closest activity is 2 (|2 - 2| = 0)
    }

    #[test]
    fn test_different_start_and_end_locations() {
        let objective = create_objective(3);

        let this_route = create_test_route_with_locations(0, vec![], Some(4));
        let other_route = create_test_route_with_locations(5, vec![6, 7, 8], Some(9));

        let distance = objective.calculate_distance(&this_route, &other_route, 2);
        assert_eq!(distance, 3.); // Closest activity is 5 (|2 - 5| = 3)
    }

    #[test]
    fn test_empty_other_route() {
        let objective = create_objective(3);

        let this_route = create_test_route_with_locations(0, vec![], Some(4));
        let other_route = create_test_route_with_locations(10, vec![], None); // Empty route, no end location

        let distance = objective.calculate_distance(&this_route, &other_route, 2);
        assert_eq!(distance, 8.);
    }

    #[test]
    fn test_from_location_outside_route() {
        let objective = create_objective(3);

        let this_route = create_test_route_with_locations(0, vec![], Some(4));
        let other_route = create_test_route_with_locations(0, vec![5, 6, 7], Some(8));

        let distance = objective.calculate_distance(&this_route, &other_route, 100);
        assert_eq!(distance, 92.0); // Closest activity is 8 (|100 - 8| = 92)
    }
}

mod calculate_dispersion_bonus {
    use super::*;

    fn calculate_expected_bonus(avg_min_distances: f64, utilization_ratio: f64) -> f64 {
        let (k, p) = (0.5, 8.);
        let utilization_weight = utilization_ratio.powf(p) + (1. - (-k * utilization_ratio).exp());
        avg_min_distances * utilization_weight
    }

    #[test]
    fn test_empty_routes() {
        let objective = create_objective(3);
        let current_route = create_test_route_with_load(vec![], 0.);
        let other_route = create_test_route_with_load(vec![], 0.);
        let routes = vec![current_route, other_route];

        let bonus = objective.calculate_dispersion_bonus(&routes, &routes[0], 4);
        assert_eq!(bonus, 0.); // No bonus for empty routes
    }

    #[test]
    fn test_single_other_route() {
        let objective = create_objective(3);
        let current_route = create_test_route_with_load(vec![], 0.);
        let other_route = create_test_route_with_load(vec![5, 6, 7], 0.5); // Load = 0.5
        let routes = vec![current_route, other_route];

        // Distance from location 4 to other_route: |4 - 5| = 1.0
        let avg_min_distances = 1.;
        let utilization_ratio = 0.5 / 1.; // Average load (only other_route)
        let expected_bonus = calculate_expected_bonus(avg_min_distances, utilization_ratio);

        let bonus = objective.calculate_dispersion_bonus(&routes, &routes[0], 4);
        assert_eq!(round(bonus), round(expected_bonus));
    }

    #[test]
    fn test_multiple_other_routes() {
        let objective = create_objective(3);
        let current_route = create_test_route_with_load(vec![], 0.);
        let other_route1 = create_test_route_with_load(vec![5, 6, 7], 0.3); // Load = 0.3
        let other_route2 = create_test_route_with_load(vec![10, 11, 12], 0.2); // Load = 0.2
        let routes = vec![current_route, other_route1, other_route2];

        // Distance from location 4 to other_route1: |4 - 5| = 1.0
        // Distance from location 4 to other_route2: |4 - 10| = 6.0
        let avg_min_distances = (1. + 6.) / 2.;
        let utilization_ratio = (0.3 + 0.2) / 2.0; // Average load (only other routes)
        let expected_bonus = calculate_expected_bonus(avg_min_distances, utilization_ratio);

        let bonus = objective.calculate_dispersion_bonus(&routes, &routes[0], 4);
        assert_eq!(bonus, expected_bonus);
    }
}

mod estimate {
    use super::*;
    use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
    use crate::helpers::models::problem::TestSingleBuilder;

    fn create_test_solution() -> SolutionContext {
        TestInsertionContextBuilder::default()
            .with_routes(vec![
                create_test_route_with_load(vec![1, 2, 3], 0.5),
                create_test_route_with_load(vec![10, 11, 12], 0.8),
            ])
            .build()
            .solution
    }

    #[test]
    fn test_estimate_route_context_with_single_job() {
        let objective = create_objective(3);
        let job = TestSingleBuilder::default().location(Some(5)).build_as_job_ref();
        let solution_ctx = create_test_solution();
        let route_ctx = &solution_ctx.routes[0];
        let move_ctx = MoveContext::Route { solution_ctx: &solution_ctx, route_ctx, job: &job };

        let cost = objective.estimate(&move_ctx);

        let expected_bonus = objective.calculate_dispersion_bonus(&solution_ctx.routes, route_ctx, 5);
        let expected_cost = -expected_bonus;
        assert_eq!(cost, expected_cost);
    }

    #[test]
    fn test_route_context_with_job_having_multiple_places() {
        let objective = create_objective(3);
        let job = TestSingleBuilder::with_locations(vec![Some(5), Some(6)]).build_as_job_ref();
        let solution_ctx = create_test_solution();
        let route_ctx = &solution_ctx.routes[0];

        let move_ctx = MoveContext::Route { solution_ctx: &solution_ctx, route_ctx: &route_ctx, job: &job };

        assert_eq!(objective.estimate(&move_ctx), Cost::default());
    }

    #[test]
    fn test_activity_context_with_single_job() {
        let objective = create_objective(3);
        let activity = ActivityBuilder::with_location(5)
            .job(Some(TestSingleBuilder::default().location(Some(5)).build_shared()))
            .build();
        let solution_ctx = create_test_solution();
        let route_ctx = &solution_ctx.routes[0];
        let act = ActivityBuilder::default().build();
        let activity_ctx = ActivityContext { index: 0, prev: &act, target: &activity, next: Some(&act) };
        let move_ctx =
            MoveContext::Activity { solution_ctx: &solution_ctx, route_ctx: &route_ctx, activity_ctx: &activity_ctx };

        let cost = objective.estimate(&move_ctx);

        assert_eq!(cost, TEST_ESTIMATE);
    }

    #[test]
    fn test_activity_context_with_job_having_multiple_places() {
        let objective = create_objective(3);
        let activity = ActivityBuilder::with_location(5)
            .job(Some(TestSingleBuilder::with_locations(vec![Some(5), Some(6)]).build_shared()))
            .build();
        let solution_ctx = create_test_solution();
        let route_ctx = &solution_ctx.routes[0];
        let act = ActivityBuilder::default().build();
        let activity_ctx = ActivityContext { index: 0, prev: &act, target: &activity, next: Some(&act) };
        let move_ctx =
            MoveContext::Activity { solution_ctx: &solution_ctx, route_ctx: &route_ctx, activity_ctx: &activity_ctx };

        let cost = objective.estimate(&move_ctx);

        let expected_bonus = objective.calculate_dispersion_bonus(&solution_ctx.routes, route_ctx, 5);
        let expected_cost = TEST_ESTIMATE - round(expected_bonus);
        assert_eq!(round(cost), expected_cost);
    }
}
