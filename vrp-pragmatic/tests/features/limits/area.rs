use crate::format::problem::Objective::*;
use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_use_constrained_areas() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
                create_delivery_job("job4", (4., 0.)),
                create_delivery_job("job5", (5., 0.)),
            ],
            areas: Some(vec![
                Area { id: "area1".to_string(), jobs: to_strings(vec!["job1", "job5"]) },
                Area { id: "area2".to_string(), jobs: to_strings(vec!["job2"]) },
            ]),
            ..create_empty_plan()
        },
        objectives: Some(vec![
            vec![AreaOrder { breaks: None, is_constrained: true, is_value_preferred: None }],
            vec![MinimizeUnassignedJobs { breaks: None }],
            vec![MinimizeCost],
        ]),
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_open_vehicle_shift()],
                limits: Some(VehicleLimits {
                    max_distance: None,
                    shift_time: None,
                    tour_size: None,
                    areas: Some(vec![
                        vec![AreaLimit { area_id: "area1".to_string(), job_value: 10. }],
                        vec![AreaLimit { area_id: "area2".to_string(), job_value: 1. }],
                    ]),
                }),
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    let tour = get_ids_from_tour(solution.tours.first().unwrap()).into_iter().flatten().collect::<Vec<_>>();
    assert_eq!(tour, to_strings(vec!["departure", "job1", "job5", "job2", "job3", "job4"]));
}

parameterized_test! {can_use_unconstrained_areas, (area1_job_value, expected_job_ids), {
    can_use_unconstrained_areas_impl(area1_job_value, expected_job_ids);
}}

can_use_unconstrained_areas! {
    case01: (2., vec!["departure", "job1", "job2", "job3", "job4"]),
    case02: (10., vec!["departure", "job1", "job5", "job4"]),
}

fn can_use_unconstrained_areas_impl(area1_job_value: f64, expected_job_ids: Vec<&str>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_demand("job1", (1., 0.), vec![2]),
                create_delivery_job_with_demand("job2", (2., 0.), vec![1]),
                create_delivery_job_with_demand("job3", (3., 0.), vec![1]),
                create_delivery_job_with_demand("job4", (4., 0.), vec![1]),
                create_delivery_job_with_demand("job5", (5., 0.), vec![2]),
            ],
            areas: Some(vec![
                Area { id: "area1".to_string(), jobs: to_strings(vec!["job1", "job5"]) },
                Area { id: "area2".to_string(), jobs: to_strings(vec!["job2", "job3", "job4"]) },
            ]),
            ..create_empty_plan()
        },
        objectives: Some(vec![
            vec![AreaOrder { breaks: None, is_constrained: false, is_value_preferred: Some(true) }],
            vec![MinimizeUnassignedJobs { breaks: None }],
            vec![MinimizeCost],
        ]),
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_open_vehicle_shift()],
                capacity: vec![5],
                limits: Some(VehicleLimits {
                    max_distance: None,
                    shift_time: None,
                    tour_size: None,
                    areas: Some(vec![
                        vec![AreaLimit { area_id: "area1".to_string(), job_value: area1_job_value }],
                        vec![AreaLimit { area_id: "area2".to_string(), job_value: 1. }],
                    ]),
                }),
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    let tour = get_ids_from_tour(solution.tours.first().unwrap()).into_iter().flatten().collect::<Vec<_>>();
    assert_eq!(tour, to_strings(expected_job_ids));
}
