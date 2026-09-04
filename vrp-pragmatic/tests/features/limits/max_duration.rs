use crate::format::problem::*;
use crate::helpers::*;
use vrp_core::prelude::Float;

fn create_vehicle_type_with_max_duration_limit(max_duration: Float) -> VehicleType {
    VehicleType {
        limits: Some(VehicleLimits {
            max_distance: None,
            max_duration: Some(max_duration),
            tour_size: None,
            min_tour_size: None,
        }),
        ..create_default_vehicle_type()
    }
}

#[test]
fn can_limit_one_job_by_max_duration() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", (100., 0.))], ..create_empty_plan() },
        fleet: Fleet { vehicles: vec![create_vehicle_type_with_max_duration_limit(99.)], ..create_default_fleet() },
        ..create_empty_problem()
    };
    let matrix = Matrix {
        profile: Some("car".to_owned()),
        timestamp: None,
        travel_times: vec![1, 100, 100, 1],
        distances: vec![1, 1, 1, 1],
        error_codes: None,
    };

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.unassigned.iter().len(), 1);
}

#[test]
fn can_skip_job_from_multiple_because_of_max_duration() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_duration("job1", (1., 0.), 10.),
                create_delivery_job_with_duration("job2", (2., 0.), 10.),
                create_delivery_job_with_duration("job3", (3., 0.), 10.),
                create_delivery_job_with_duration("job4", (4., 0.), 10.),
                create_delivery_job_with_duration("job5", (5., 0.), 10.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet { vehicles: vec![create_vehicle_type_with_max_duration_limit(40.)], ..create_default_fleet() },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // Five jobs of 10s each on a line out of the depot; a 40s cap fits three of them and the round
    // trip costs the same in either direction, so the visit order is a tie the solver may break
    // either way. The claim is which three fit and why the other two do not.
    assert_eq!(solution.tours.len(), 1);
    assert!(solution.statistic.duration <= 40, "the cap must hold: {}", solution.statistic.duration);
    assert_eq!(solution.statistic.distance, 6);

    let mut served = served_job_ids(&solution.tours[0]);
    served.sort();
    assert_eq!(served, ["job1", "job2", "job3"].map(String::from));

    let unassigned = solution.unassigned.expect("job4 and job5 must be reported as unassigned");
    let mut unassigned_ids = unassigned.iter().map(|job| job.job_id.clone()).collect::<Vec<_>>();
    unassigned_ids.sort();
    assert_eq!(unassigned_ids, ["job4", "job5"].map(String::from));

    assert!(
        unassigned.iter().all(|job| {
            job.reasons.iter().any(|reason| {
                reason.code == "MAX_DURATION_CONSTRAINT"
                    && reason.details.as_ref().is_some_and(|details| {
                        details.iter().any(|detail| detail.vehicle_id == "my_vehicle_1" && detail.shift_index == 0)
                    })
            })
        }),
        "the cap must be the stated reason, against the vehicle that could not take them: {unassigned:?}"
    );
}

#[test]
fn can_serve_job_when_it_starts_late() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (1., 0.), vec![(100, 200)], 10.)],
            ..create_empty_plan()
        },
        fleet: Fleet { vehicles: vec![create_vehicle_type_with_max_duration_limit(50.)], ..create_default_fleet() },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert!(!solution.tours.is_empty());
}
