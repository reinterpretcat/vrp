use crate::format::problem::Objective::*;
use crate::format::problem::*;
use crate::format::solution::{UnassignedJob, UnassignedJobDetail, UnassignedJobReason};
use crate::helpers::*;

parameterized_test! {can_prefer_jobs_with_more_value, objectives, {
    can_prefer_jobs_with_more_value_impl(objectives);
}}

can_prefer_jobs_with_more_value! {
    case01: Some(vec![
        vec![MinimizeUnassignedJobs { breaks: None }],
        vec![MaximizeValue { reduction_factor: Some(0.1), breaks: None }],
        vec![MinimizeCost],
    ]),
    case02: None,
}

fn can_prefer_jobs_with_more_value_impl(objectives: Option<Vec<Vec<Objective>>>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (1., 0.)), create_delivery_job_with_value("job2", (2., 0.), 100.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType { capacity: vec![1], ..create_default_vehicle_type() }],
            ..create_default_fleet()
        },
        objectives,
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.tours.len(), 1);
    let unassigned = solution.unassigned.unwrap();
    assert_eq!(unassigned.len(), 1);
    let unassigned = unassigned.first().cloned().unwrap();
    assert_eq!(
        unassigned,
        UnassignedJob {
            job_id: "job1".to_string(),
            reasons: vec![UnassignedJobReason {
                code: "CAPACITY_CONSTRAINT".to_string(),
                description: "does not fit into any vehicle due to capacity".to_string(),
                details: Some(vec![UnassignedJobDetail { vehicle_id: "my_vehicle_1".to_string(), shift_index: 0 }])
            }]
        }
    );
}
