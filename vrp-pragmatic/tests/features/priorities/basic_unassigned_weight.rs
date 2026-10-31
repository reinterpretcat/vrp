use crate::format::problem::Objective::*;
use crate::format::problem::*;
use crate::helpers::*;
use vrp_core::utils::Float;

fn solve_one_of_two(far_job_weight: Option<Float>) -> Vec<String> {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("near", (1., 0.)),
                Job { unassigned_weight: far_job_weight, ..create_delivery_job("far", (5., 0.)) },
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType { capacity: vec![1], ..create_default_vehicle_type() }],
            ..create_default_fleet()
        },
        objectives: Some(vec![MinimizeUnassigned { breaks: None }, MinimizeCost]),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    solution.unassigned.unwrap_or_default().into_iter().map(|job| job.job_id).collect()
}

#[test]
fn can_keep_the_cheaper_job_when_both_weigh_the_same() {
    assert_eq!(solve_one_of_two(None), vec!["far".to_string()]);
}

#[test]
fn can_keep_the_heavier_job_assigned_over_a_cheaper_one() {
    assert_eq!(solve_one_of_two(Some(2.)), vec!["near".to_string()]);
}
