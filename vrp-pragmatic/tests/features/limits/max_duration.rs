use crate::format::problem::*;
use crate::format_time;
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

/// A required break is not a job but reserved time: it is charged only when an activity's schedule
/// or a travel leg runs into its window, which makes the charge depend on the schedule. Here the
/// break at 50 (60s long) is out of reach while the tour holds job1 alone, and is swallowed whole by
/// job2's service once job2 joins the tour. The claim is that the cap weighs that break.
///
/// Without job2 the tour runs 0 -> job1 (1..11) -> depot, 12s long. With job2 the vehicle waits at
/// job2's location from 12 until its window opens at 55, serves it for 100s and, because that
/// stretch covers the reserved window, pays 55 more seconds of break on top - departing at 210 and
/// reaching the depot at 212, nearly twice the 160s cap.
#[test]
fn can_account_for_break_that_only_a_second_job_reaches() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (1., 0.), vec![(0, 1000)], 10.),
                create_delivery_job_with_times("job2", (2., 0.), vec![(55, 1000)], 100.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    // pin the departure so that the tour cannot buy itself room by leaving later
                    start: ShiftStart {
                        earliest: format_time(0.),
                        latest: Some(format_time(0.)),
                        location: (0., 0.).to_loc(),
                    },
                    breaks: Some(vec![VehicleBreak::Required {
                        time: VehicleRequiredBreakTime::ExactTime {
                            earliest: format_time(50.),
                            latest: format_time(50.),
                        },
                        duration: 60.,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_vehicle_type_with_max_duration_limit(160.)
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.tours.len(), 1);
    assert!(
        solution.statistic.duration <= 160,
        "the cap must hold once the break is inside the tour: {}",
        solution.statistic.duration
    );
    assert_eq!(served_job_ids(&solution.tours[0]), ["job1"].map(String::from));

    let unassigned = solution.unassigned.expect("job2 does not fit under the cap and must be reported");
    assert_eq!(unassigned.iter().map(|job| job.job_id.clone()).collect::<Vec<_>>(), ["job2".to_string()]);
    assert!(
        unassigned[0].reasons.iter().any(|reason| reason.code == "MAX_DURATION_CONSTRAINT"),
        "the cap must be the stated reason: {unassigned:?}"
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
