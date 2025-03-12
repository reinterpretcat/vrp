use super::*;
use crate::{
    helpers::construction::heuristics::TestInsertionContextBuilder,
    helpers::models::solution::*,
    models::{common::*, problem::TravelTime, solution::Route},
};

struct MockTransport(Vec<Vec<Cost>>);

impl TransportCost for MockTransport {
    fn distance_approx(&self, _: &Profile, from: Location, to: Location) -> Cost {
        self.0[from][to]
    }

    fn distance(&self, _: &Route, _: Location, _: Location, _: TravelTime) -> Cost {
        unreachable!()
    }

    fn duration(&self, _: &Route, _: Location, _: Location, _: TravelTime) -> Cost {
        unreachable!()
    }

    fn duration_approx(&self, _: &Profile, _: Location, _: Location) -> Duration {
        unreachable!()
    }

    fn size(&self) -> usize {
        unreachable!()
    }
}

fn create_test_route_ctx(locations: &[usize], end: Location) -> RouteContext {
    RouteContextBuilder::default()
        .with_route(
            RouteBuilder::default()
                .with_start(ActivityBuilder::with_location(0).job(None).build())
                .with_end(ActivityBuilder::with_location(end).job(None).build())
                .add_activities(locations.iter().map(|&i| ActivityBuilder::with_location(i).build()))
                .build(),
        )
        .build()
}

fn create_matrix_from_locations(locations: &[(f64, f64)]) -> Vec<Vec<Cost>> {
    let n = locations.len();
    let mut matrix = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in 0..n {
            if i != j {
                let (x1, y1) = locations[i];
                let (x2, y2) = locations[j];
                // Euclidean distance
                matrix[i][j] = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
            }
        }
    }

    matrix
}

fn get_locations(route_ctx: &RouteContext) -> Vec<Location> {
    route_ctx.route().tour.all_activities().map(|a| a.place.location).collect()
}

#[cfg(test)]
mod cost_matrix_tests {
    use super::*;

    #[rustfmt::skip]
    fn create_matrix_data() -> Vec<Vec<Cost>> {
        // simulate: (0., 0.), (2., 0.), (2., 1.), (0., 1.)
        vec![
            vec![0.0, 2.0, 2.2, 1.0],
            vec![2.0, 0.0, 1.0, 2.2],
            vec![2.2, 1.0, 0.0, 2.0],
            vec![1.0, 2.2, 2.0, 0.0],
        ]
    }

    parameterized_test! {test_cost_matrix_new, (end_location, expected_count), {
        test_cost_matrix_new_impl(end_location, expected_count);
    }}

    test_cost_matrix_new! {
        case01_same_end: (0, 4),
        case02_diff_end: (3, 5),
    }

    fn test_cost_matrix_new_impl(end_location: usize, expected_count: usize) {
        let route_ctx = create_test_route_ctx(&[1, 2, 3], end_location);
        let transport = MockTransport(create_matrix_data());

        let cost_matrix = CostMatrix::new(&route_ctx, &transport);

        assert_eq!(cost_matrix.locations.len(), expected_count);
        assert_eq!(cost_matrix.neighbourhood.len(), expected_count);
        assert!(
            cost_matrix.neighbourhood.iter().all(|n| n.len() == expected_count - 1),
            "each node should have {} neighbors (all other nodes)",
            expected_count - 1
        );
    }

    #[test]
    fn test_cost_matrix_neighbors_are_sorted() {
        let route_ctx = create_test_route_ctx(&[1, 2, 3], 0);
        let transport = MockTransport(create_matrix_data());

        let cost_matrix = CostMatrix::new(&route_ctx, &transport);

        assert_eq!(cost_matrix.locations.len(), 4);
        assert_eq!(cost_matrix.neighbourhood.len(), 4);
        assert_eq!(cost_matrix.neighbours(0), &[3, 1, 2]);
        assert_eq!(cost_matrix.neighbours(1), &[2, 0, 3]);
        assert_eq!(cost_matrix.neighbours(2), &[1, 3, 0]);
        assert_eq!(cost_matrix.neighbours(3), &[0, 2, 1]);
    }

    #[test]
    fn test_edge_cost_calculation() {
        let route_ctx = create_test_route_ctx(&[1, 2, 3], 0);
        let transport = MockTransport(create_matrix_data());

        let cost_matrix = CostMatrix::new(&route_ctx, &transport);

        assert_eq!(cost_matrix.cost(&(0, 1)), 2.0); // (0,0) to (2,0)
        assert_eq!(cost_matrix.cost(&(0, 2)), 2.2); // (0,0) to (2,1)
        assert_eq!(cost_matrix.cost(&(1, 3)), 2.2); // (2,0) to (0,1)
        assert_eq!(cost_matrix.cost(&(2, 3)), 2.0); // (2,1) to (0,1)
    }
}

mod route_path_tests {
    use super::*;

    parameterized_test! {test_route_to_path, (expected_path, end_location), {
        test_route_to_path_impl(expected_path, end_location);
    }}

    test_route_to_path! {
        case01_diff_end: (4, vec![0, 1, 2, 3, 4]),
        case02_same_end: (0, vec![0, 1, 2, 3]),
    }

    fn test_route_to_path_impl(end_location: Location, expected_path: Vec<Location>) {
        let route_ctx = create_test_route_ctx(&[1, 2, 3], end_location);

        let path = route_to_path(&route_ctx);

        assert_eq!(path, expected_path);
    }

    parameterized_test! {test_rearrange_route, (optimized_path, end_location), {
        test_rearrange_route_impl(optimized_path, end_location);
    }}

    test_rearrange_route! {
        case01_diff_end: (vec![0, 2, 1, 3, 4], 4),
        case02_same_end: (vec![0, 3, 1, 2, 0], 0),
        case03_reversed_diff_end: (vec![0, 4, 3, 2, 1], 4),
        case04_reversed_same_end: (vec![0, 3, 2, 1, 0], 0),
        case05_already_optimal_same_end: (vec![0, 1, 2, 3, 0], 0),
        case06_already_optimal_diff_end: (vec![0, 1, 2, 3, 4], 4),
    }

    fn test_rearrange_route_impl(optimized_path: Vec<Location>, end_location: Location) {
        let mut route_ctx = create_test_route_ctx(&[1, 2, 3], end_location);

        rearrange_route(&mut route_ctx, optimized_path.clone());

        assert_eq!(get_locations(&route_ctx), optimized_path);
    }
}

mod optimize_route_tests {
    use super::*;

    #[test]
    fn test_optimize_route_trivial_small() {
        // route with just 3 points - should not be optimized as per the function's logic
        let mut route_ctx = create_test_route_ctx(&[1, 2], 0);
        let locations = [(0., 0.), (10., 0.), (5., 8.)]; // depot, 2 locations

        optimize_route(&mut route_ctx, &MockTransport(create_matrix_from_locations(&locations)));

        assert_eq!(route_to_path(&route_ctx), vec![0, 1, 2], "route should remain unchanged");
    }

    #[test]
    fn test_optimize_route_empty_route() {
        // test with minimal route that has just start and end (same location)
        let mut route_ctx = create_test_route_ctx(&[], 0);

        // should return early without changes
        optimize_route(&mut route_ctx, &MockTransport(create_matrix_from_locations(&[(0., 0.)])));

        // verify route is untouched
        assert_eq!(route_ctx.route().tour.total(), 2);
    }

    #[test]
    fn test_optimize_route_simple_tsp() {
        let mut route_ctx = create_test_route_ctx(&[2, 1, 3], 0);
        let locations = [(0., 0.), (1., 0.), (1., 1.), (0., 1.)]; // depot, 3 locations

        optimize_route(&mut route_ctx, &MockTransport(create_matrix_from_locations(&locations)));

        assert_eq!(route_to_path(&route_ctx), vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_optimize_route_simple_tsp_with_different_end() {
        let mut route_ctx = create_test_route_ctx(&[1, 2, 3], 4);
        // 3(0,1) 4(1,1)-2(2,1)
        //    |   \         |
        // 0(0,0)        1(2,0)
        let locations = [(0., 0.), (2., 0.), (2., 1.), (0., 1.), (1., 1.)]; // depot, 3 locations, different end

        optimize_route(&mut route_ctx, &MockTransport(create_matrix_from_locations(&locations)));

        assert_eq!(get_locations(&route_ctx), &[0, 3, 1, 2, 4]);
    }

    #[test]
    fn test_optimize_route_circle_tsp() {
        // test with 10 cities in a circle, where optimal solution is the circle path
        let locations: Vec<(f64, f64)> = (0..10)
            .map(|i| {
                let angle = 2.0 * std::f64::consts::PI * (i as f64) / 10.0;
                (angle.cos() * 10.0, angle.sin() * 10.0)
            })
            .collect();
        // create route with cities in scrambled order to test optimization
        let mut route_ctx = create_test_route_ctx(&[1, 7, 4, 2, 9, 6, 8, 3, 5], 0);

        optimize_route(&mut route_ctx, &MockTransport(create_matrix_from_locations(&locations)));

        assert_eq!(get_locations(&route_ctx), &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
    }

    #[test]
    fn test_optimize_route_no_improvement_possible() {
        // create an already-optimal route to verify no changes are made
        let mut route_ctx = create_test_route_ctx(&[1, 2, 3], 0);
        // create a matrix where the optimal path is already the current path
        let mut matrix_data = vec![vec![10.; 5]; 5];
        // modify the matrix to make the existing path optimal: make sequential path very cheap
        (0..4).for_each(|i| {
            matrix_data[i][(i + 1) % 4] = 1.;
            matrix_data[(i + 1) % 4][i] = 1.;
        });

        optimize_route(&mut route_ctx, &MockTransport(matrix_data));

        assert_eq!(get_locations(&route_ctx), &[0, 1, 2, 3, 0]);
    }

    #[test]
    fn test_optimize_route_edge_case_different_start_end() {
        // Edge case where start and end are different locations
        let mut route_ctx = create_test_route_ctx(&[1, 4, 2, 3], 5);
        let locations = [(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 1.0), (4.0, 0.0), (5.0, 0.0)];

        optimize_route(&mut route_ctx, &MockTransport(create_matrix_from_locations(&locations)));

        assert_eq!(get_locations(&route_ctx), &[0, 1, 2, 3, 5, 4], "should move end");
    }

    #[test]
    fn test_optimize_route_identical_locations() {
        let mut route_ctx = create_test_route_ctx(&[1, 3, 2, 2, 1, 3], 0);
        let locations = [(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)];

        optimize_route(&mut route_ctx, &MockTransport(create_matrix_from_locations(&locations)));

        assert_eq!(get_locations(&route_ctx), &[0, 1, 1, 2, 2, 3, 3, 0]);
    }
}

mod search_tests {
    use rosomaxa::utils::CollectGroupBy;
    use std::sync::Arc;

    use super::*;
    use crate::{
        construction::heuristics::UnassignmentInfo,
        helpers::{
            models::{
                domain::{ProblemBuilder, TestGoalContextBuilder, test_random},
                problem::*,
            },
            solver::create_default_refinement_ctx,
        },
        models::{problem::*, solution::Registry},
    };

    #[derive(Clone)]
    struct Tour {
        vehicle_id: &'static str,
        shift_locations: (Location, Location),
        activities: Vec<(Location, Duration, (Timestamp, Timestamp))>,
    }

    fn create_insertion_context(
        activities: &[(Location, Duration, (Timestamp, Timestamp))],
        end: Location,
    ) -> InsertionContext {
        let activities = activities.to_vec();
        create_insertion_context_with_toures(&[Tour { vehicle_id: "v1", shift_locations: (0, end), activities }])
    }

    fn create_insertion_context_with_toures(tours: &[Tour]) -> InsertionContext {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicles(
                tours
                    .iter()
                    .map(|Tour { vehicle_id, shift_locations, .. }| {
                        TestVehicleBuilder::default()
                            .id(vehicle_id)
                            .details(vec![VehicleDetail {
                                start: Some(VehiclePlace {
                                    location: shift_locations.0,
                                    time: TimeInterval { earliest: Some(0.), latest: None },
                                }),
                                end: Some(VehiclePlace {
                                    location: shift_locations.1,
                                    time: TimeInterval { earliest: None, latest: Some(100.) },
                                }),
                            }])
                            .build()
                    })
                    .collect(),
            )
            .build();
        let fleet = Arc::new(fleet);

        let jobs_activities = tours
            .iter()
            .flat_map(|Tour { vehicle_id, activities, .. }| {
                activities.iter().map(|&(location, duration, (start, end))| {
                    let job = TestSingleBuilder::default()
                        .id(&location.to_string())
                        .location(Some(location))
                        .duration(duration)
                        .times(vec![TimeWindow::new(start, end)])
                        .build_as_job_ref();
                    let activity =
                        ActivityBuilder::with_location_tw_and_duration(location, TimeWindow::new(start, end), duration)
                            .job(Some(job.to_single().clone()))
                            .build();
                    (*vehicle_id, job, activity)
                })
            })
            .collect::<Vec<_>>();

        let problem = ProblemBuilder::default()
            .with_goal(TestGoalContextBuilder::with_transport_feature().build())
            .with_jobs(jobs_activities.iter().map(|(_, job, _)| job.clone()).collect())
            .build();

        let tour_activities = jobs_activities.into_iter().collect_group_by_key(|(vehicle_id, _, _)| *vehicle_id);

        let mut insertion_ctx = TestInsertionContextBuilder::default()
            .with_problem(problem)
            .with_fleet(fleet.clone())
            .with_registry(Registry::new(&fleet, test_random()))
            .with_routes(
                tour_activities
                    .into_iter()
                    .map(|(vehicle_id, activities)| {
                        RouteContextBuilder::default()
                            .with_route(
                                RouteBuilder::default()
                                    .with_vehicle(&fleet, vehicle_id)
                                    .add_activities(activities.into_iter().map(|(_, _, activity)| activity))
                                    .build(),
                            )
                            .build()
                    })
                    .collect(),
            )
            .build();

        let goal = insertion_ctx.problem.goal.clone();
        goal.accept_solution_state(&mut insertion_ctx.solution);

        insertion_ctx
    }

    fn get_route_locations(ctx: &InsertionContext) -> Vec<Vec<Location>> {
        ctx.solution.routes.iter().map(get_locations).collect()
    }

    #[test]
    fn test_search_diverse_mode_repairs_routes() {
        let original_ctx = create_insertion_context(&[(10, 0., (0., 5.)), (20, 5., (30., 40.)), (5, 0., (0., 10.))], 0);
        let refinement_ctx = create_default_refinement_ctx(original_ctx.problem.clone());

        let new_ctx = LKHSearch::new(LKHSearchMode::Diverse).search(&refinement_ctx, &original_ctx);

        assert_eq!(get_route_locations(&new_ctx), vec![vec![0, 20, 0]],);
        assert_eq!(new_ctx.solution.unassigned.len(), 2);
    }

    #[test]
    fn test_search_improvement_mode_keep_original() {
        let original_ctx = create_insertion_context(&[(10, 0., (0., 5.)), (20, 5., (30., 40.)), (5, 0., (0., 10.))], 0);
        let refinement_ctx = create_default_refinement_ctx(original_ctx.problem.clone());

        let new_ctx = LKHSearch::new(LKHSearchMode::ImprovementOnly).search(&refinement_ctx, &original_ctx);

        assert_eq!(get_route_locations(&new_ctx), vec![vec![0, 10, 20, 5, 0]],);
        assert_eq!(new_ctx.solution.unassigned.len(), 0);
    }

    #[test]
    fn test_search_different_end_must_be_kept_last() {
        let original_ctx =
            create_insertion_context(&[(30, 0., (0., 100.)), (20, 0., (0., 100.)), (40, 0., (0., 100.))], 10);
        let refinement_ctx = create_default_refinement_ctx(original_ctx.problem.clone());

        let new_ctx = LKHSearch::new(LKHSearchMode::Diverse).search(&refinement_ctx, &original_ctx);

        assert_eq!(get_route_locations(&new_ctx), vec![vec![0, 20, 30, 40, 10]],);
        assert_eq!(new_ctx.solution.unassigned.len(), 0);
    }

    parameterized_test! {test_repair_routes_handles_removed_route, reversed, {
        test_repair_routes_handles_removed_route_impl(reversed);
    }}

    test_repair_routes_handles_removed_route! {
        case01_direct: false,
        case02_reversed: true,
    }

    fn test_repair_routes_handles_removed_route_impl(reversed: bool) {
        let tours = vec![
            Tour {
                vehicle_id: "v1",
                shift_locations: (0, 100),
                activities: vec![(20, 0., (0., 10.)), (30, 0., (0., 10.))],
            },
            Tour { vehicle_id: "v2", shift_locations: (0, 0), activities: vec![(1, 0., (0., 1000.))] },
        ];

        let mut orig_solution = create_insertion_context_with_toures(&tours);
        orig_solution.solution.routes.sort_by(|a, b| {
            a.route().actor.vehicle.dimens.get_vehicle_id().cmp(&b.route().actor.vehicle.dimens.get_vehicle_id())
        });

        let mut new_solution = orig_solution.deep_copy();
        let idx = if reversed {
            new_solution.solution.routes.reverse();
            1
        } else {
            0
        };
        new_solution.solution.unassigned.insert(
            new_solution.solution.routes[idx].route_mut().tour.remove_activity_at(2),
            UnassignmentInfo::Unknown,
        );

        let result_ctx = LKHSearch::new(LKHSearchMode::ImprovementOnly).repair_routes(new_solution, &orig_solution);

        assert_eq!(get_route_locations(&result_ctx), vec![vec![0, 1, 0], vec![0, 20, 30, 100]]);
        assert!(result_ctx.solution.registry.next_route().next().is_none());
    }
}
