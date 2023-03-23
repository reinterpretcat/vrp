use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_have_empty_detail_in_empty_solution() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_skills(
                "job1",
                (1., 0.),
                all_of_skills(vec!["unique_skill".to_string()]),
            )],
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic::default(),
            tours: vec![],
            unassigned: Some(vec![UnassignedJob {
                job_id: "job1".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: "SKILL_CONSTRAINT".to_string(),
                    description: "cannot serve required skill".to_string(),
                    details: None
                }]
            }]),
            ..create_empty_solution()
        }
    );
}
