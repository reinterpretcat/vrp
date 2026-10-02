use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;
use crate::{format_time, parse_time};

fn window(earliest: f64, latest: f64) -> VisitWindowJson {
    VisitWindowJson { earliest: format_time(earliest), latest: format_time(latest) }
}

fn problem(jobs: Vec<Job>, recurring: (f64, f64), non_recurring: (f64, f64), overflow: bool) -> Problem {
    Problem {
        plan: Plan { jobs, ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    visit_windows: Some(VisitWindowsJson {
                        recurring: Some(window(recurring.0, recurring.1)),
                        non_recurring: Some(window(non_recurring.0, non_recurring.1)),
                        overflow: Some(overflow),
                    }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    }
}

fn tagged(id: &str, x: f64, tag: &str) -> Job {
    Job { visit_window: Some(tag.to_string()), ..create_delivery_job_with_duration(id, (x, 0.), 10.) }
}

fn service_start(solution: &Solution, job_id: &str) -> f64 {
    solution
        .tours
        .iter()
        .flat_map(|tour| tour.stops.iter())
        .find(|stop| stop.activities().iter().any(|activity| activity.job_id == job_id))
        .map(|stop| parse_time(&stop.schedule().departure) - 10.)
        .unwrap_or_else(|| panic!("{job_id} is not planned"))
}

fn solve(problem: Problem) -> Solution {
    let matrix = create_matrix_from_problem(&problem);
    solve_with_metaheuristic(problem, Some(vec![matrix]))
}

#[test]
fn without_overflow_a_late_recurring_visit_stays_unassigned() {
    let solution = solve(problem(
        vec![tagged("near", 1., "recurring"), tagged("far", 30., "recurring")],
        (0., 20.),
        (0., 100.),
        false,
    ));

    let unassigned = solution.unassigned.expect("far does not fit the recurring window");
    assert_eq!(unassigned.len(), 1);
    assert_eq!(unassigned[0].job_id, "far");
}

#[test]
fn with_overflow_the_late_recurring_visit_is_planned() {
    let solution = solve(problem(
        vec![tagged("near", 1., "recurring"), tagged("far", 30., "recurring")],
        (0., 20.),
        (0., 100.),
        true,
    ));

    assert!(solution.unassigned.is_none(), "overflow places far in the non-recurring window");
    assert!(service_start(&solution, "near") + 10. <= 20.);
}

#[test]
fn a_non_recurring_visit_keeps_to_its_window() {
    let solution = solve(problem(vec![tagged("late", 30., "non-recurring")], (0., 100.), (50., 100.), false));

    assert!(service_start(&solution, "late") >= 50.);
}

#[test]
fn overflow_waits_for_the_own_window_when_the_visit_fits_it() {
    // Serving at once in the non-recurring window would be shorter than waiting for 50.
    let solution = solve(problem(vec![tagged("a", 1., "recurring")], (50., 70.), (0., 100.), true));

    assert!(solution.unassigned.is_none());
    assert!(service_start(&solution, "a") >= 50.);
}

#[test]
fn an_untagged_visit_is_not_bound_by_the_windows() {
    let solution =
        solve(problem(vec![create_delivery_job_with_duration("fixed", (90., 0.), 10.)], (0., 20.), (0., 50.), false));

    assert!(solution.unassigned.is_none());
    assert!(service_start(&solution, "fixed") >= 90.);
}

#[test]
fn overflow_never_wins_on_cost_alone() {
    // On an open route A(10) → O(15) → B(20) is the shortest, but serves B 40–50, past its window
    // ending at 45. A → B → O is longer and keeps B inside. The overflow objective ranks above
    // cost, so the longer tour wins.
    let mut problem = problem(
        vec![tagged("a", 10., "recurring"), tagged("o", 15., "non-recurring"), tagged("b", 20., "recurring")],
        (0., 45.),
        (0., 200.),
        true,
    );
    problem.fleet.vehicles[0].shifts[0] = VehicleShift {
        visit_windows: problem.fleet.vehicles[0].shifts[0].visit_windows.clone(),
        ..create_default_open_vehicle_shift()
    };

    let solution = solve(problem);

    assert!(solution.unassigned.is_none());
    assert!(service_start(&solution, "b") + 10. <= 45., "b overflowed to save distance");
}
