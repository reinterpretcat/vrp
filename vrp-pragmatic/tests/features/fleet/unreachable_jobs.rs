use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_use_vehicle_with_open_end() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", (1., 0.))], ..create_empty_plan() },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let matrix = Matrix {
        profile: Some("car".to_owned()),
        timestamp: None,
        travel_times: vec![0, 1, 1, 0],
        distances: vec![0, 1, 1, 0],
        error_codes: Some(vec![0, 1, 1, 1]),
    };

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.tours.is_empty(), "the job cannot be reached at all");

    // Only that unreachability is among the reasons — see the note in `job_times`.
    let unassigned = solution.unassigned.expect("job1 must be reported as unassigned");
    assert_eq!(unassigned.len(), 1);
    assert_eq!(unassigned[0].job_id, "job1");
    assert!(
        unassigned[0].reasons.iter().any(|reason| reason.code == "REACHABLE_CONSTRAINT"),
        "unreachability must be given as a reason: {:?}",
        unassigned[0].reasons
    );
}
