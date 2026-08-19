use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_unassign_multi_job_due_to_capacity() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_multi_job(
                "multi",
                vec![((2., 0.), 1., vec![2]), ((8., 0.), 1., vec![1])],
                vec![((6., 0.), 1., vec![3])],
            )],
            ..create_empty_plan()
        },
        fleet: Fleet { vehicles: vec![create_vehicle_with_capacity("my_vehicle", vec![2])], ..create_default_fleet() },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.tours.is_empty(), "the multi job does not fit any vehicle");

    // Only that capacity is among the reasons — see the note in `job_times`.
    let unassigned = solution.unassigned.expect("the multi job must be reported as unassigned");
    assert_eq!(unassigned.len(), 1);
    assert_eq!(unassigned[0].job_id, "multi");
    assert!(
        unassigned[0].reasons.iter().any(|reason| reason.code == "CAPACITY_CONSTRAINT"),
        "capacity must be given as a reason: {:?}",
        unassigned[0].reasons
    );
}
