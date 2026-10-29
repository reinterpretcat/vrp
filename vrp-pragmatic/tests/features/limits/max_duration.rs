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

/// `maxDuration` caps the time the shift is PAID for, and `costs.span` says which stretch of the
/// tour that is. So a tour can sit well inside its cap and still run far longer door to door: the
/// commute here is 100s at each end and the work between the two jobs is 12s, against a cap of 100.
///
/// The solver places both jobs, and `solve_with_metaheuristic` runs the solution through the
/// checker — which is the point of doing this end to end. The checker read `statistic.duration`,
/// always the round trip, and called this tour broken at 212 against a cap the solver had honoured.
#[test]
fn can_solve_and_check_a_tour_whose_commute_exceeds_a_span_trimmed_cap() {
    let mut vehicle = create_vehicle_type_with_max_duration_limit(100.);
    vehicle.costs.span = Some(RouteCostSpan::FirstJobToLastJob);

    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (100., 0.)), create_delivery_job("job2", (100., 10.))],
            ..create_empty_plan()
        },
        fleet: Fleet { vehicles: vec![vehicle], ..create_default_fleet() },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert_eq!(solution.tours.len(), 1);
    // the round trip is what the checker used to read, and it is nowhere near the cap
    assert!(solution.tours[0].statistic.duration > 100, "expect a tour whose commute breaks the cap");
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

/// The idle stretch in front of a tour's first job is free only because the departure moves forward
/// once the job is in. On a shift with reserved time it may not move: leaving later can put a break
/// inside the service instead of the idle, pushing work past the end of its own window, and
/// `advance_departure_time` then puts the departure straight back. The tour keeps the whole idle
/// stretch, and the cap has to be read against that.
///
/// Two breaks: a 5s one due at 90, and a 60s one due at 105. The job's window is one second wide at
/// 100. Departing at 0 the vehicle idles until 100, and the first break is spent idling and costs
/// nothing; the tour ends at 181 because the second break falls on the drive home. Departing at 99
/// instead would take the second break inside the job's service and finish the work at 160, long
/// past the window that closes at 101 - so that departure is refused and never happens. A cap of 150
/// must be read against the 181 the route really runs, not the 82 the later departure suggests.
#[test]
fn can_refuse_a_later_departure_a_break_makes_infeasible() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (1., 0.), vec![(100, 101)], 20.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: format_time(0.),
                        latest: Some(format_time(500.)),
                        location: (0., 0.).to_loc(),
                    },
                    breaks: Some(vec![
                        VehicleBreak::Required {
                            time: VehicleRequiredBreakTime::ExactTime {
                                earliest: format_time(90.),
                                latest: format_time(90.),
                            },
                            duration: 5.,
                        },
                        VehicleBreak::Required {
                            time: VehicleRequiredBreakTime::ExactTime {
                                earliest: format_time(105.),
                                latest: format_time(105.),
                            },
                            duration: 60.,
                        },
                    ]),
                    ..create_default_vehicle_shift()
                }],
                ..create_vehicle_type_with_max_duration_limit(150.)
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(
        solution.tours.iter().all(|tour| tour.statistic.duration <= 150),
        "no tour may run past the cap: {:?}",
        solution.tours.iter().map(|tour| tour.statistic.duration).collect::<Vec<_>>()
    );
    let unassigned = solution.unassigned.expect("job1 does not fit under the cap and must be reported");
    assert_eq!(unassigned.iter().map(|job| job.job_id.clone()).collect::<Vec<_>>(), ["job1".to_string()]);
}

/// Moving a departure forward normally shortens a tour, so only the receding side ever weighed the
/// duration limit. With reserved time it can lengthen one, and the two callers that move a departure
/// (the `AdvanceDeparture` post-processing and the reschedule local operator) both run after the
/// duration constraint has had its last word. A tour pushed past its cap there would never be seen
/// again.
///
/// Both breaks are needed. Departing at 0 the vehicle idles at job1 until 60, so the 20s break due
/// at 50 is half spent idling and costs 10; job1 leaves at 75, job2 is served 76..81, and the tour
/// ends at 83 - inside the 100s cap. The whole-tour advance wants to depart at 59 instead, which
/// takes the idle away: the first break is then charged in full, job1 leaves at 85, and the drive to
/// job2 runs into the 100s break due at 84. That tour ends at 193, running 134 from its later
/// departure.
#[test]
fn can_refuse_a_later_departure_that_a_break_makes_longer() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (1., 0.), vec![(60, 1000)], 5.),
                create_delivery_job_with_times("job2", (2., 0.), vec![(76, 1000)], 5.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: format_time(0.),
                        latest: Some(format_time(500.)),
                        location: (0., 0.).to_loc(),
                    },
                    breaks: Some(vec![
                        VehicleBreak::Required {
                            time: VehicleRequiredBreakTime::ExactTime {
                                earliest: format_time(50.),
                                latest: format_time(50.),
                            },
                            duration: 20.,
                        },
                        VehicleBreak::Required {
                            time: VehicleRequiredBreakTime::ExactTime {
                                earliest: format_time(84.),
                                latest: format_time(84.),
                            },
                            duration: 100.,
                        },
                    ]),
                    ..create_default_vehicle_shift()
                }],
                ..create_vehicle_type_with_max_duration_limit(100.)
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(
        solution.tours.iter().all(|tour| tour.statistic.duration <= 100),
        "no tour may be pushed past the cap by a later departure: {:?}",
        solution.tours.iter().map(|tour| tour.statistic.duration).collect::<Vec<_>>()
    );
    assert!(solution.unassigned.is_none(), "both jobs fit at the departure the route keeps");
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

/// A required break due right as the day's last job ends is priced on the drive home: reserved time
/// is charged at the latest start of its window, and the job (200..260) only touches the window
/// starting at 260. On a span that ends at the last job that drive is unpaid, so the tour is paid for
/// 60s against a cap of 400. `RescheduleReservedTime` used to move the break off the road into the
/// job's departure afterwards, which put all 500s of it inside the paid stretch: the checker read
/// 560 against the cap, with the solver long past the point of weighing it. This is the shape of a
/// production tour (a 6h block due at 13:00 behind a 12:00..13:00 job) that came out 432s over.
#[test]
fn can_keep_break_due_after_last_job_out_of_span_ending_at_last_job() {
    for span in [RouteCostSpan::FirstJobToLastJob, RouteCostSpan::DepotToLastJob] {
        let mut vehicle = VehicleType {
            shifts: vec![VehicleShift {
                breaks: Some(vec![VehicleBreak::Required {
                    time: VehicleRequiredBreakTime::ExactTime {
                        earliest: format_time(259.),
                        latest: format_time(260.),
                    },
                    duration: 500.,
                }]),
                ..create_default_vehicle_shift()
            }],
            ..create_vehicle_type_with_max_duration_limit(400.)
        };
        vehicle.costs.span = Some(span.clone());
        let problem = Problem {
            plan: Plan {
                jobs: vec![create_delivery_job_with_times("job1", (20., 0.), vec![(200, 300)], 60.)],
                ..create_empty_plan()
            },
            fleet: Fleet { vehicles: vec![vehicle], ..create_default_fleet() },
            ..create_empty_problem()
        };
        let matrix = create_matrix_from_problem(&problem);

        // the checker runs here, and it is what reported the break as paid time
        let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

        assert!(solution.unassigned.is_none(), "{span:?}: job1 fits the cap on the stretch it is paid for");
        assert_eq!(served_job_ids(&solution.tours[0]), ["job1".to_string()]);
        let job_stop = &solution.tours[0].stops[1];
        assert_eq!(job_stop.schedule().departure, format_time(260.), "{span:?}: the break must stay on the drive home");
    }
}
