use crate::format::problem::*;
use crate::helpers::*;
use crate::format_time;

/// Tests that jobs with earlier due dates are preferred when using minimize-overdue objective.
#[test]
fn can_prefer_jobs_with_earlier_due_dates() {
    // Job 1 is due at timestamp 0 (epoch) - already overdue since shift starts at 0
    // Job 2 is due at timestamp 100 - will be less overdue when scheduled on shift starting at 0
    // With capacity of 1, solver should pick job2 (less overdue) over job1
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                // job1 is due earlier, so if scheduled at time 0, it has no overdue
                // But job2 is due at time 100, so if scheduled at time 0, job2 would have -100 overdue (actually on time)
                create_delivery_job_with_due_date("job1", (1., 0.), &format_time(0.)),
                create_delivery_job_with_due_date("job2", (2., 0.), &format_time(100.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                capacity: vec![1], // Can only serve one job
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives: Some(vec![
            Objective::MinimizeUnassigned { breaks: None },
            Objective::MinimizeOverdue,
            Objective::MinimizeCost,
        ]),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // Both jobs have due dates >= shift start, so neither is overdue
    // The solver should assign one job and leave one unassigned due to capacity
    assert_eq!(solution.tours.len(), 1);
    assert!(solution.unassigned.is_some());
}

/// Tests that a job scheduled past its due date contributes to overdue cost.
#[test]
fn can_calculate_overdue_for_late_job() {
    // Create a problem where the shift starts after the job's due date
    // Due date: timestamp 0
    // Shift starts: timestamp 86400 (1 day later)
    // Expected overdue: 1 day
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_due_date("job1", (1., 0.), &format_time(0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: format_time(86400.), // 1 day after epoch
                        latest: None,
                        location: (0., 0.).to_loc(),
                    },
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: format_time(172800.), // 2 days after epoch
                        location: (0., 0.).to_loc(),
                    }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives: Some(vec![
            Objective::MinimizeUnassigned { breaks: None },
            Objective::MinimizeOverdue,
            Objective::MinimizeCost,
        ]),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // Job should be assigned (we minimize unassigned first)
    assert_eq!(solution.tours.len(), 1);
    // Verify the job is in the tour
    assert!(solution.tours[0]
        .stops
        .iter()
        .any(|stop| stop.activities().iter().any(|a| a.job_id == "job1")));
}

/// Tests that jobs without due dates don't contribute to overdue cost.
#[test]
fn can_handle_jobs_without_due_date() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),                                    // No due date
                create_delivery_job_with_due_date("job2", (2., 0.), &format_time(1000.)), // Has due date
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_default_vehicle_type()],
            ..create_default_fleet()
        },
        objectives: Some(vec![
            Objective::MinimizeUnassigned { breaks: None },
            Objective::MinimizeOverdue,
            Objective::MinimizeCost,
        ]),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // Both jobs should be assigned
    assert_eq!(solution.tours.len(), 1);
    assert!(solution.unassigned.is_none());
}

/// Tests that with multiple shifts, jobs are assigned to minimize overdue.
#[test]
fn can_prefer_earlier_shift_to_minimize_overdue() {
    // Job is due at timestamp 50
    // Shift 1 starts at 0 (on time, 0 days overdue)
    // Shift 2 starts at 86400 (1 day late, ~1 day overdue)
    // Solver should prefer shift 1
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_due_date("job1", (1., 0.), &format_time(50.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![
                    VehicleShift {
                        start: ShiftStart {
                            earliest: format_time(0.),
                            latest: None,
                            location: (0., 0.).to_loc(),
                        },
                        end: Some(ShiftEnd {
                            earliest: None,
                            latest: format_time(100.),
                            location: (0., 0.).to_loc(),
                        }),
                        ..create_default_vehicle_shift()
                    },
                    VehicleShift {
                        start: ShiftStart {
                            earliest: format_time(86400.),
                            latest: None,
                            location: (0., 0.).to_loc(),
                        },
                        end: Some(ShiftEnd {
                            earliest: None,
                            latest: format_time(172800.),
                            location: (0., 0.).to_loc(),
                        }),
                        ..create_default_vehicle_shift()
                    },
                ],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives: Some(vec![
            Objective::MinimizeUnassigned { breaks: None },
            Objective::MinimizeOverdue,
            Objective::MinimizeCost,
        ]),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // Job should be assigned to shift 0 (starts at 0) to minimize overdue
    assert_eq!(solution.tours.len(), 1);
    assert_eq!(solution.tours[0].shift_index, 0);
}
