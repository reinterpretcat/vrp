use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;
use crate::{format_time, parse_time};

fn window(earliest: f64, latest: f64, fallback: Option<&str>) -> VisitWindowJson {
    VisitWindowJson {
        earliest: format_time(earliest),
        latest: format_time(latest),
        fallback: fallback.map(str::to_string),
        bridge: false,
    }
}

/// One shift with an `own` window that falls back to a `fallback` window when `overflow` is on.
fn problem(jobs: Vec<Job>, own: (f64, f64), fallback: (f64, f64), overflow: bool) -> Problem {
    Problem {
        plan: Plan { jobs, ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    visit_windows: Some(
                        [
                            ("own".to_string(), window(own.0, own.1, overflow.then_some("fallback"))),
                            ("fallback".to_string(), window(fallback.0, fallback.1, None)),
                        ]
                        .into_iter()
                        .collect(),
                    ),
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
fn without_fallback_a_late_visit_stays_unassigned() {
    let solution =
        solve(problem(vec![tagged("near", 1., "own"), tagged("far", 30., "own")], (0., 20.), (0., 100.), false));

    let unassigned = solution.unassigned.expect("far does not fit the recurring window");
    assert_eq!(unassigned.len(), 1);
    assert_eq!(unassigned[0].job_id, "far");
}

#[test]
fn with_fallback_the_late_visit_is_planned() {
    let solution =
        solve(problem(vec![tagged("near", 1., "own"), tagged("far", 30., "own")], (0., 20.), (0., 100.), true));

    assert!(solution.unassigned.is_none(), "overflow places far in the non-recurring window");
    assert!(service_start(&solution, "near") + 10. <= 20.);
}

#[test]
fn a_visit_tagged_with_the_fallback_keeps_to_it() {
    let solution = solve(problem(vec![tagged("late", 30., "fallback")], (0., 100.), (50., 100.), false));

    assert!(service_start(&solution, "late") >= 50.);
}

#[test]
fn overflow_waits_for_the_own_window_when_the_visit_fits_it() {
    // Serving at once in the non-recurring window would be shorter than waiting for 50.
    let solution = solve(problem(vec![tagged("a", 1., "own")], (50., 70.), (0., 100.), true));

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
        vec![tagged("a", 10., "own"), tagged("o", 15., "fallback"), tagged("b", 20., "own")],
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

#[test]
fn a_visit_held_back_by_its_window_mid_tour_reports_the_wait() {
    // "early" has to be served by 5, so the tour is under way long before the recurring window
    // opens at 50: the vehicle reaches "held" around 12 and waits there. The tour has to say
    // service began at 50, or the checker cannot match the activity to its job and reads the
    // visit as served before its window.
    let solution = solve(problem(
        vec![create_delivery_job_with_times("early", (1., 0.), vec![(0, 5)], 10.), tagged("held", 2., "own")],
        (50., 70.),
        (0., 100.),
        false,
    ));

    assert!(solution.unassigned.is_none());
    let held = solution
        .tours
        .iter()
        .flat_map(|tour| tour.stops.iter())
        .flat_map(|stop| stop.activities().iter())
        .find(|activity| activity.job_id == "held")
        .expect("held is planned");
    let time = held.time.as_ref().expect("a held visit keeps its own service time");
    assert_eq!(parse_time(&time.start), 50.);
    assert_eq!(parse_time(&time.end), 60.);
}

/// A vehicle type whose shift has a `regular` window 50–100, bridged, falling back to `working` 0–200.
fn bridged_vehicle(id: &str, skills: &[&str]) -> VehicleType {
    VehicleType {
        type_id: id.to_string(),
        vehicle_ids: vec![format!("{id}_1")],
        skills: Some(skills.iter().map(|skill| skill.to_string()).collect()),
        shifts: vec![VehicleShift {
            visit_windows: Some(
                [
                    (
                        "regular".to_string(),
                        VisitWindowJson {
                            earliest: format_time(50.),
                            latest: format_time(100.),
                            fallback: Some("working".to_string()),
                            bridge: true,
                        },
                    ),
                    ("working".to_string(), window(0., 200., None)),
                ]
                .into_iter()
                .collect(),
            ),
            ..create_default_vehicle_shift()
        }],
        ..create_default_vehicle_type()
    }
}

fn bridged_problem(jobs: Vec<Job>, vehicles: Vec<VehicleType>) -> Problem {
    Problem {
        plan: Plan { jobs, ..create_empty_plan() },
        fleet: Fleet { vehicles, ..create_default_fleet() },
        ..create_empty_problem()
    }
}

fn fixed(id: &str, x: f64, times: (i32, i32), skill: &str) -> Job {
    Job {
        skills: Some(all_of_skills(vec![skill.to_string()])),
        ..create_delivery_job_with_times(id, (x, 0.), vec![times], 10.)
    }
}

fn regular(id: &str, x: f64, skill: &str) -> Job {
    Job { skills: Some(all_of_skills(vec![skill.to_string()])), ..tagged(id, x, "regular") }
}

#[test]
fn a_regular_visit_fills_the_gap_to_an_early_fixed_visit() {
    // the fixed visit is served by 5: the route is out long before the regular hours open at 50,
    // and the regular visit fills the gap instead of waiting for them.
    let solution = solve(bridged_problem(
        vec![fixed("fixed", 1., (0, 5), "a"), regular("filler", 2., "a")],
        vec![bridged_vehicle("tech", &["a"])],
    ));

    assert!(solution.unassigned.is_none());
    assert!(service_start(&solution, "filler") < 50., "the filler waited for the regular hours");
}

#[test]
fn a_technician_without_a_fixed_visit_keeps_the_regular_hours() {
    // the fixed visit can only go to `a`; `b` has none, so its regular visit keeps to 50–100 even
    // though serving it at once would be shorter.
    let solution = solve(bridged_problem(
        vec![fixed("fixed", 1., (0, 5), "a"), regular("other", 2., "b")],
        vec![bridged_vehicle("a", &["a"]), bridged_vehicle("b", &["b"])],
    ));

    assert!(solution.unassigned.is_none());
    assert!(service_start(&solution, "other") >= 50., "a technician without a fixed visit worked early");
}
