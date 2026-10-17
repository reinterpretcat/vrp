use crate::format::problem::*;
use crate::format::solution::Solution;
use crate::helpers::*;
use crate::{format_time, parse_time};

const DAY: f64 = 86400.;

/// A visit of rhythm `sub-1` at the depot's neighbour, open on the given days.
fn create_test_job(id: &str, order: usize, days: std::ops::RangeInclusive<i64>) -> Job {
    Job {
        services: Some(vec![JobTask {
            places: vec![JobPlace {
                location: (1., 0.).to_loc(),
                duration: 10.,
                times: Some(
                    days.map(|day| vec![format_time(day as f64 * DAY), format_time(day as f64 * DAY + 1000.)])
                        .collect(),
                ),
                tag: None,
            }],
            demand: None,
            order: None,
            due_date: None,
        }]),
        spacing: Some(JobSpacing {
            group: "sub-1".to_string(),
            order,
            interval_days: 7,
            tolerance_days: 1,
            anchor: None,
        }),
        ..create_job(id)
    }
}

/// One vehicle working every day of the fortnight.
fn create_test_problem(jobs: Vec<Job>) -> Problem {
    Problem {
        plan: Plan { jobs, ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: (0..15)
                    .map(|day| VehicleShift {
                        start: ShiftStart {
                            earliest: format_time(day as f64 * DAY),
                            latest: None,
                            location: (0., 0.).to_loc(),
                        },
                        end: Some(ShiftEnd {
                            earliest: None,
                            latest: format_time(day as f64 * DAY + 2000.),
                            location: (0., 0.).to_loc(),
                        }),
                        ..create_default_vehicle_shift()
                    })
                    .collect(),
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    }
}

fn served_days(solution: &Solution) -> Vec<(String, i64)> {
    let mut days = solution
        .tours
        .iter()
        .flat_map(|tour| {
            tour.stops
                .iter()
                .flat_map(|stop| stop.activities().iter())
                .filter(|activity| activity.job_id != "departure" && activity.job_id != "arrival")
                .map(move |activity| {
                    (activity.job_id.clone(), (parse_time(&tour.stops[0].schedule().departure) / DAY).floor() as i64)
                })
        })
        .collect::<Vec<_>>();
    days.sort();

    days
}

/// Without the spacing both visits share the cheapest tour: the first day.
#[test]
fn can_keep_the_second_visit_an_interval_after_the_first() {
    let problem = create_test_problem(vec![create_test_job("first", 1, 0..=1), create_test_job("second", 2, 0..=14)]);
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    let days = served_days(&solution);
    let gap = days[1].1 - days[0].1;
    assert!((6..=8).contains(&gap), "visits must be a week apart, give or take a day, got {days:?}");
}

#[test]
fn can_leave_a_visit_unassigned_when_no_day_keeps_its_interval() {
    let problem = create_test_problem(vec![create_test_job("first", 1, 0..=0), create_test_job("second", 2, 1..=3)]);
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // either may give way; its reason names whichever shift refused it last, the solver's reasons
    // are not ranked
    assert_eq!(served_days(&solution).len(), 1);
    assert_eq!(solution.unassigned.map(|unassigned| unassigned.len()), Some(1));
}

#[test]
fn can_place_an_anchor_whatever_its_neighbours() {
    let mut anchor = create_test_job("anchor", 2, 1..=1);
    anchor.spacing.as_mut().unwrap().anchor = Some(true);
    let problem = create_test_problem(vec![create_test_job("first", 1, 0..=0), anchor]);
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    let days = served_days(&solution);
    assert!(days.iter().any(|(id, day)| id == "anchor" && *day == 1), "the anchor keeps its day, got {days:?}");
}
