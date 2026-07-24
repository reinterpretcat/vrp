use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

fn create_vehicle_with_job_time_constraints(earliest_first: Option<f64>, latest_last: Option<f64>) -> VehicleType {
    create_named_vehicle_with_job_time_constraints("my_vehicle", earliest_first, latest_last)
}

fn create_named_vehicle_with_job_time_constraints(
    type_id: &str,
    earliest_first: Option<f64>,
    latest_last: Option<f64>,
) -> VehicleType {
    VehicleType {
        type_id: type_id.to_string(),
        vehicle_ids: vec![format!("{}_1", type_id)],
        shifts: vec![VehicleShift {
            start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
            end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (0., 0.).to_loc() }),
            breaks: None,
            reloads: None,
            recharges: None,
            job_times: Some(JobTimeConstraints {
                earliest_first: earliest_first.map(format_time),
                latest_last: latest_last.map(format_time),
            }),
        }],
        ..create_default_vehicle_type()
    }
}

fn create_open_route_vehicle_with_job_time_constraints(
    earliest_first: Option<f64>,
    latest_last: Option<f64>,
) -> VehicleType {
    VehicleType {
        shifts: vec![VehicleShift {
            start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
            end: None, // Open route - no return to depot
            breaks: None,
            reloads: None,
            recharges: None,
            job_times: Some(JobTimeConstraints {
                earliest_first: earliest_first.map(format_time),
                latest_last: latest_last.map(format_time),
            }),
        }],
        ..create_default_vehicle_type()
    }
}

#[test]
fn can_reject_job_when_arrival_before_earliest_first() {
    // Job is at location (5, 0), so arrival at 5 time units
    // earliest_first is 10, so job cannot start before 10
    // Job time window ends at 8, which is before earliest_first
    // Therefore, the job should be unassigned
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (5., 0.), vec![(0, 8)], 1.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_job_time_constraints(Some(10.), None)],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .unassigned(Some(vec![UnassignedJob {
                job_id: "job1".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: "JOB_TIME_CONSTRAINT".to_string(),
                    description: "cannot be assigned due to shift job time constraints".to_string(),
                    details: None
                }]
            }]))
            .build()
    );
}

#[test]
fn can_assign_job_when_arrival_after_earliest_first() {
    // Job is at location (15, 0), so arrival at 15 time units
    // earliest_first is 10, so job can start at 15 (which is after 10)
    // Job should be assigned
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (15., 0.), vec![(0, 100)], 1.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_job_time_constraints(Some(10.), None)],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none(), "Job should be assigned");
    assert_eq!(solution.tours.len(), 1);
    assert!(
        solution.tours[0].stops.iter().any(|stop| { stop.activities().iter().any(|a| a.job_id == "job1") }),
        "Tour should contain job1"
    );
}

#[test]
fn can_assign_job_when_time_window_allows_waiting() {
    // Job is at location (5, 0), so arrival at 5 time units
    // earliest_first is 10, but job time window extends to 100
    // The job should be assigned because the time window allows waiting until earliest_first
    // Note: The constraint checks feasibility; actual schedule timing may vary
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (5., 0.), vec![(0, 100)], 1.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_job_time_constraints(Some(10.), None)],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // The key assertion: job should be assigned (not rejected) because the constraint
    // recognizes that the job's time window (ends at 100) extends past earliest_first (10)
    assert!(solution.unassigned.is_none(), "Job should be assigned because TW allows waiting");
    assert_eq!(solution.tours.len(), 1);
    assert!(
        solution.tours[0].stops.iter().any(|stop| { stop.activities().iter().any(|a| a.job_id == "job1") }),
        "Tour should contain job1"
    );
}

#[test]
fn can_reject_job_when_departure_after_latest_last() {
    // Job is at location (50, 0), so arrival at 50 time units
    // With service duration of 1, departure is at 51
    // latest_last is 30, so departure 51 > 30
    // Job should be unassigned
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (50., 0.), vec![(0, 100)], 1.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_job_time_constraints(None, Some(30.))],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .unassigned(Some(vec![UnassignedJob {
                job_id: "job1".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: "JOB_TIME_CONSTRAINT".to_string(),
                    description: "cannot be assigned due to shift job time constraints".to_string(),
                    details: None
                }]
            }]))
            .build()
    );
}

#[test]
fn can_assign_job_when_departure_before_latest_last() {
    // Job is at location (10, 0), so arrival at 10 time units
    // With service duration of 1, departure is at 11
    // latest_last is 30, so departure 11 < 30
    // Job should be assigned
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (10., 0.), vec![(0, 100)], 1.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_job_time_constraints(None, Some(30.))],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none(), "Job should be assigned");
    assert_eq!(solution.tours.len(), 1);
    assert!(
        solution.tours[0].stops.iter().any(|stop| { stop.activities().iter().any(|a| a.job_id == "job1") }),
        "Tour should contain job1"
    );
}

#[test]
fn can_apply_both_constraints_and_assign_job() {
    // Job is at location (15, 0), so arrival at 15 time units
    // earliest_first is 10 (arrival 15 >= 10, OK)
    // With service duration of 1, departure is at 16
    // latest_last is 30 (departure 16 <= 30, OK)
    // Job should be assigned
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (15., 0.), vec![(0, 100)], 1.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_job_time_constraints(Some(10.), Some(30.))],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none(), "Job should be assigned");
    assert_eq!(solution.tours.len(), 1);
}

#[test]
fn can_apply_both_constraints_and_reject_job() {
    // Job is at location (5, 0), so arrival at 5 time units
    // earliest_first is 10, job TW ends at 8
    // Cannot wait until earliest_first because TW ends before that
    // Job should be unassigned
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (5., 0.), vec![(0, 8)], 1.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_job_time_constraints(Some(10.), Some(100.))],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_some(), "Job should be unassigned");
    assert_eq!(solution.unassigned.as_ref().unwrap()[0].job_id, "job1");
}

#[test]
fn can_handle_multiple_jobs_with_constraints() {
    // Three jobs:
    // - job1 at (5, 0): arrival 5, can wait until 10 (earliest_first), departure ~11
    // - job2 at (20, 0): arrival ~31 (from job1), departure ~32
    // - job3 at (100, 0): arrival too late for latest_last (50)
    //
    // With earliest_first=10 and latest_last=50:
    // - job1 and job2 should be assigned
    // - job3 should be unassigned (departure would be > 50)
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (5., 0.), vec![(0, 100)], 1.),
                create_delivery_job_with_times("job2", (20., 0.), vec![(0, 100)], 1.),
                create_delivery_job_with_times("job3", (100., 0.), vec![(0, 200)], 1.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_job_time_constraints(Some(10.), Some(50.))],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // job3 should be unassigned
    assert!(solution.unassigned.is_some(), "Should have unassigned jobs");
    let unassigned_ids: Vec<_> = solution.unassigned.as_ref().unwrap().iter().map(|u| u.job_id.as_str()).collect();
    assert!(unassigned_ids.contains(&"job3"), "job3 should be unassigned");

    // job1 and job2 should be assigned
    assert_eq!(solution.tours.len(), 1);
    let assigned_jobs: Vec<_> =
        solution.tours[0].stops.iter().flat_map(|stop| stop.activities().iter()).map(|a| a.job_id.as_str()).collect();
    assert!(assigned_jobs.contains(&"job1"), "job1 should be assigned");
    assert!(assigned_jobs.contains(&"job2"), "job2 should be assigned");
}

#[test]
fn can_reject_job_when_duration_causes_departure_after_latest_last() {
    // Job is at location (10, 0), so arrival at 10 time units
    // latest_last is 15, so arrival (10) is BEFORE latest_last - seems OK
    // BUT service duration is 10, meaning:
    // - Service starts at 10, finishes at 20
    // - Departure (20) > latest_last (15)
    // Job should be unassigned because departure exceeds latest_last
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (10., 0.), vec![(0, 100)], 10.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_job_time_constraints(None, Some(15.))],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // Job should be unassigned - even though arrival < latest_last,
    // the service duration causes departure to exceed latest_last
    assert!(solution.unassigned.is_some(), "Job should be unassigned");
    assert_eq!(solution.unassigned.as_ref().unwrap()[0].job_id, "job1");
}

#[test]
fn can_work_with_depot_to_depot_span() {
    // Verify that job_times works independently of cost span
    // Even with depot-to-depot costing, the job time constraints should apply
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (5., 0.), vec![(0, 8)], 1.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (0., 0.).to_loc() }),
                    breaks: None,
                    reloads: None,
                    recharges: None,
                    job_times: Some(JobTimeConstraints { earliest_first: Some(format_time(10.)), latest_last: None }),
                }],
                costs: VehicleCosts {
                    fixed: Some(10.),
                    distance: 1.,
                    time: 1.,
                    span: Some(RouteCostSpan::DepotToDepot), // Explicit depot-to-depot
                },
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // Job should still be rejected due to job_times constraint
    assert!(solution.unassigned.is_some(), "Job should be unassigned");
    assert_eq!(solution.unassigned.as_ref().unwrap()[0].reasons[0].code, "JOB_TIME_CONSTRAINT");
}

#[test]
fn can_apply_different_constraints_to_different_vehicles() {
    // Two vehicles with different job_times constraints:
    // - vehicle_strict: earliest_first=20, latest_last=25 (very strict)
    // - vehicle_relaxed: earliest_first=5, latest_last=100 (relaxed)
    //
    // Job at (10, 0): arrival at 10, departure at 11
    // - vehicle_strict: arrival 10 < earliest_first 20 AND can't wait (need to check TW)
    // - vehicle_relaxed: arrival 10 > earliest_first 5, departure 11 < latest_last 100
    //
    // Job should be assigned to vehicle_relaxed
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (10., 0.), vec![(0, 15)], 1.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![
                create_named_vehicle_with_job_time_constraints("vehicle_strict", Some(20.), Some(25.)),
                create_named_vehicle_with_job_time_constraints("vehicle_relaxed", Some(5.), Some(100.)),
            ],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none(), "Job should be assigned");
    assert_eq!(solution.tours.len(), 1);
    // Job should be assigned to vehicle_relaxed
    assert!(
        solution.tours[0].vehicle_id.contains("vehicle_relaxed"),
        "Job should be assigned to vehicle_relaxed, but was assigned to {}",
        solution.tours[0].vehicle_id
    );
}

#[test]
fn can_reject_job_when_waiting_causes_latest_last_violation() {
    // This tests the interaction between earliest_first and latest_last
    // Job at (5, 0): arrival at 5
    // earliest_first=10: must wait until 10 to start
    // Service duration=5: depart at 15
    // latest_last=12: departure 15 > 12
    //
    // The waiting required by earliest_first causes a latest_last violation
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (5., 0.), vec![(0, 100)], 5.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_job_time_constraints(Some(10.), Some(12.))],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // Job should be unassigned because waiting until earliest_first (10)
    // plus service duration (5) = departure at 15 > latest_last (12)
    assert!(solution.unassigned.is_some(), "Job should be unassigned");
    assert_eq!(solution.unassigned.as_ref().unwrap()[0].job_id, "job1");
}

#[test]
fn earliest_first_only_applies_to_first_job() {
    // Two jobs:
    // - job1 at (15, 0): arrival 15, satisfies earliest_first=10
    // - job2 at (5, 0): would arrive at 5 if it were first, but it's second
    //
    // After job1 (at 15, depart ~16), travel to job2 (at 5,0)
    // Distance from (15,0) to (5,0) is 10, so arrive at job2 around 26
    // earliest_first should NOT apply to job2 since it's the second job
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (15., 0.), vec![(0, 100)], 1.),
                create_delivery_job_with_times("job2", (5., 0.), vec![(0, 100)], 1.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_job_time_constraints(Some(10.), None)],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // Both jobs should be assigned
    assert!(solution.unassigned.is_none(), "Both jobs should be assigned");
    assert_eq!(solution.tours.len(), 1);
    let assigned_jobs: Vec<_> =
        solution.tours[0].stops.iter().flat_map(|stop| stop.activities().iter()).map(|a| a.job_id.as_str()).collect();
    assert!(assigned_jobs.contains(&"job1"), "job1 should be assigned");
    assert!(assigned_jobs.contains(&"job2"), "job2 should be assigned");
}

#[test]
fn latest_last_only_applies_to_last_job() {
    // Two jobs where only one can be last:
    // - job1 at (10, 0): close to depot
    // - job2 at (50, 0): far from depot, departure would be 51
    //
    // latest_last=30 means departure from last job must be <= 30
    // If job2 is last: departure 51 > 30, violates latest_last
    // If job1 is last: departure ~11 < 30, OK
    //
    // The solver should find a route where job1 is last (job2 -> job1)
    // Or reject job2 if it can't find a valid ordering
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (10., 0.), vec![(0, 100)], 1.),
                create_delivery_job_with_times("job2", (25., 0.), vec![(0, 100)], 1.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            // latest_last=30 means we need to finish the last job by 30
            vehicles: vec![create_vehicle_with_job_time_constraints(None, Some(30.))],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // Both jobs should be assigned with proper ordering
    assert!(solution.unassigned.is_none(), "Both jobs should be assigned");
    assert_eq!(solution.tours.len(), 1);

    // Get the order of jobs in the tour
    let job_order: Vec<_> = solution.tours[0]
        .stops
        .iter()
        .flat_map(|stop| stop.activities().iter())
        .filter(|a| a.activity_type == "delivery")
        .map(|a| a.job_id.as_str())
        .collect();

    assert_eq!(job_order.len(), 2, "Should have 2 jobs in tour");
}

#[test]
fn can_handle_tight_time_window() {
    // Very tight window: earliest_first=10, latest_last=12
    // Job at (10, 0): arrival 10, service 1, departure 11
    // 10 >= earliest_first (10) and 11 <= latest_last (12)
    // Just barely fits
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (10., 0.), vec![(0, 100)], 1.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_job_time_constraints(Some(10.), Some(12.))],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none(), "Job should fit in tight window");
    assert_eq!(solution.tours.len(), 1);
}

#[test]
fn can_reject_job_outside_tight_time_window() {
    // Very tight window: earliest_first=10, latest_last=11
    // Job at (10, 0): arrival 10, service 2, departure 12
    // 10 >= earliest_first (10) but 12 > latest_last (11)
    // Doesn't fit
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (10., 0.), vec![(0, 100)], 2.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_job_time_constraints(Some(10.), Some(11.))],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_some(), "Job should not fit in tight window");
    assert_eq!(solution.unassigned.as_ref().unwrap()[0].job_id, "job1");
}

#[test]
fn can_apply_latest_last_on_open_route() {
    // Open route (no return to depot) with latest_last constraint
    // Job at (10, 0): arrival 10, service 1, departure 11
    // latest_last=15: departure 11 < 15, OK
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (10., 0.), vec![(0, 100)], 1.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_open_route_vehicle_with_job_time_constraints(None, Some(15.))],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none(), "Job should be assigned on open route");
    assert_eq!(solution.tours.len(), 1);
}

#[test]
fn can_reject_job_on_open_route_when_latest_last_violated() {
    // Open route with latest_last constraint
    // Job at (20, 0): arrival 20, service 1, departure 21
    // latest_last=15: departure 21 > 15, rejected
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (20., 0.), vec![(0, 100)], 1.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_open_route_vehicle_with_job_time_constraints(None, Some(15.))],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_some(), "Job should be rejected on open route");
    assert_eq!(solution.unassigned.as_ref().unwrap()[0].job_id, "job1");
}
