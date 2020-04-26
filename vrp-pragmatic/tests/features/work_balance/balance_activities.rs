use crate::format::problem::Objective::*;
use crate::format::problem::*;
use crate::format::solution::Tour;
use crate::helpers::*;

fn get_activities_count(tour: &Tour) -> usize {
    tour.stops
        .iter()
        .map(|stop| stop.activities.iter().filter(|activity| activity.activity_type == "delivery").count())
        .sum()
}

parameterized_test! {can_balance_activities_with_threshold, (threshold, expected_lowest), {
    can_balance_activities_with_threshold_impl(threshold, expected_lowest);
}}

can_balance_activities_with_threshold! {
    case01: (None, 3),
    case02: (Some(2), 3),
    case03: (Some(5), 2),
}

fn can_balance_activities_with_threshold_impl(threshold: Option<usize>, expected_lowest: usize) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1.0", vec![1., 0.]),
                create_delivery_job("job1.1", vec![1., 0.]),
                create_delivery_job("job1.2", vec![1., 0.]),
                create_delivery_job("job1.3", vec![1., 0.]),
                create_delivery_job("job2.0", vec![2., 0.]),
                create_delivery_job("job2.1", vec![2., 0.]),
            ],
            relations: None,
        },
        fleet: Fleet {
            vehicles: vec![
                VehicleType {
                    vehicle_ids: vec!["my_vehicle1".to_string()],
                    shifts: vec![create_default_open_vehicle_shift()],
                    capacity: vec![4],
                    ..create_default_vehicle_type()
                },
                VehicleType {
                    type_id: "my_vehicle2".to_string(),
                    vehicle_ids: vec!["my_vehicle2".to_string()],
                    shifts: vec![create_default_vehicle_shift_with_locations((3., 0.), (3., 0.))],
                    capacity: vec![4],
                    ..create_default_vehicle_type()
                },
            ],
            profiles: create_default_profiles(),
        },
        objectives: Some(Objectives {
            primary: vec![BalanceActivities { threshold }],
            secondary: Some(vec![MinimizeCost]),
        }),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.tours.len(), 2);
    assert_eq!(solution.tours.iter().map(get_activities_count).min().unwrap(), expected_lowest);
    assert_eq!(solution.tours.iter().map(get_activities_count).max().unwrap(), 6 - expected_lowest);
}
