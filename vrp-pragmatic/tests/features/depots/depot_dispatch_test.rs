use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

fn create_problem_with_depots(vehicle_ids: Vec<&str>, depots: Option<Vec<VehicleDepot>>) -> Problem {
    Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![2., 0.]),
                create_delivery_job("job2", vec![2., 0.]),
                create_delivery_job("job3", vec![2., 0.]),
                create_delivery_job("job4", vec![2., 0.]),
                create_delivery_job("job5", vec![2., 0.]),
            ],
            relations: None,
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vehicle_ids.iter().map(|id| id.to_string()).collect(),
                shifts: vec![VehicleShift { depots, ..create_default_vehicle_shift() }],
                capacity: vec![1],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
        },
        ..create_empty_problem()
    }
}

fn assert_tours(tours: &[Tour], values: (f64, f64, f64)) {
    tours.iter().for_each(|tour| {
        assert_eq!(tour.stops.len(), 4);

        assert_eq!(tour.stops[0].time.departure, format_time(values.0));
        assert_eq!(tour.stops[0].activities.len(), 1);
        assert_eq!(tour.stops[0].activities[0].activity_type, "departure");

        assert_eq!(tour.stops[1].activities.len(), 1);
        assert_eq!(tour.stops[1].activities[0].activity_type, "depot");
        assert_eq!(tour.stops[1].time.arrival, format_time(values.1));
        assert_eq!(tour.stops[1].time.departure, format_time(values.2));
    });
}

#[test]
fn can_dispatch_multiple_vehicles_at_single_depot() {
    let problem = create_problem_with_depots(
        vec!["v1", "v2", "v3", "v4", "v5"],
        Some(vec![VehicleDepot {
            location: vec![1., 0.].to_loc(),
            dispatch: vec![
                VehicleDispatch { max: 2, start: format_time(10.), end: format_time(12.) },
                VehicleDispatch { max: 3, start: format_time(13.), end: format_time(16.) },
            ],
            tag: None,
        }]),
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert_eq!(solution.tours.len(), 5);

    assert_tours(&solution.tours[0..2], (9., 10., 12.));
    assert_tours(&solution.tours[2..5], (12., 13., 16.));
}
