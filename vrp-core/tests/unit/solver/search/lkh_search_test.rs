use super::*;
use crate::{
    helpers::models::solution::*,
    models::{common::Duration, problem::TravelTime, solution::Route},
};

struct MockTransport(Vec<Vec<Cost>>);

impl TransportCost for MockTransport {
    fn distance_approx(&self, _: &Profile, from: Location, to: Location) -> Cost {
        self.0[from as usize][to as usize]
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
