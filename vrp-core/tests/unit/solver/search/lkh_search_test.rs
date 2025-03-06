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

        let locations: Vec<_> = route_ctx.route().tour.all_activities().map(|a| a.place.location).collect();
        assert_eq!(locations, optimized_path);
    }
}
