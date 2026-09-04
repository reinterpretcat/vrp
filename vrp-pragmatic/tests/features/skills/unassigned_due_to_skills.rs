use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_have_unassigned_due_to_missing_vehicle_skill() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_skills(
                "job1",
                (1., 0.),
                all_of_skills(vec!["unique_skill".to_string()]),
            )],
            ..create_empty_plan()
        },
        fleet: Fleet { vehicles: vec![create_default_vehicle("vehicle_without_skill")], ..create_default_fleet() },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.tours.is_empty(), "the only vehicle cannot serve the job");

    // Only that the missing skill is among the reasons: which constraint is reported first depends
    // on the order the routes happened to be tried, so the list is not an ordering to pin.
    let unassigned = solution.unassigned.expect("job1 must be reported as unassigned");
    assert_eq!(unassigned.len(), 1);
    assert_eq!(unassigned[0].job_id, "job1");
    assert!(
        unassigned[0].reasons.iter().any(|reason| reason.code == "SKILL_CONSTRAINT"),
        "the missing skill must be given as a reason: {:?}",
        unassigned[0].reasons
    );
}
