use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_have_multiple_unassigned_reasons() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_demand("job1", (1., 0.), vec![9]),
                create_delivery_job_with_demand("job2", (1., 0.), vec![9]),
                create_delivery_job_with_skills("job3", (1., 0.), all_of_skills(vec!["unique_skill".to_string()])),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_default_vehicle("vehicle1"), create_default_vehicle("vehicle2")],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.tours.len(), 2);
    assert!(solution.unassigned.is_some());
    assert_eq!(
        solution.unassigned,
        Some(vec![UnassignedJob {
            job_id: "job3".to_string(),
            reasons: vec![UnassignedJobReason {
                code: "SKILL_CONSTRAINT".to_string(),
                description: "cannot serve required skill".to_string(),
                details: Some(vec![
                    UnassignedJobDetail { vehicle_id: "vehicle1_1".to_string(), shift_index: 0 },
                    UnassignedJobDetail { vehicle_id: "vehicle2_1".to_string(), shift_index: 0 }
                ])
            }]
        }])
    );
}
