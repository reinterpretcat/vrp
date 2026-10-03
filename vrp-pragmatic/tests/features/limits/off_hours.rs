use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;
use vrp_core::prelude::Float;

/// Four jobs of 100 seconds apiece, one per unit along a line out of the depot, and two vehicles
/// which could take them - the overtime test's problem, priced on the clock instead of the duration.
fn create_problem(regular_until: Option<Float>, off_hours: Option<Float>) -> Problem {
    Problem {
        plan: Plan {
            jobs: (1..=4)
                .map(|idx| create_delivery_job_with_duration(&format!("job{idx}"), (idx as Float, 0.), 100.))
                .collect(),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                costs: VehicleCosts { off_hours, ..create_default_vehicle_costs() },
                shifts: vec![VehicleShift {
                    regular_hours: regular_until
                        .map(|latest| RegularHoursJson { earliest: format_time(0.), latest: format_time(latest) }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives: create_min_jobs_cost_objective(),
    }
}

fn solve(regular_until: Option<Float>, off_hours: Option<Float>) -> crate::format::solution::Solution {
    let problem = create_problem(regular_until, off_hours);
    let matrix = create_matrix_from_problem(&problem);

    solve_with_metaheuristic(problem, Some(vec![matrix]))
}

/// One tour of 408 seconds costs 426 while time is all the same. With regular hours ending at 204
/// and an off-hours rate of 5 against a regular 1, its last 204 seconds cost 816 more; split in two,
/// the tours end at 204 and 208, so only 4 seconds lie outside: 444 + 16 = 460.
#[test]
fn can_split_a_tour_that_would_run_past_the_regular_hours() {
    let solution = solve(Some(204.), Some(5.));

    assert!(solution.unassigned.is_none(), "all jobs must be served: {:?}", solution.unassigned);
    assert_eq!(solution.tours.len(), 2, "the work must be spread over both shifts: {solution:?}");
    assert_eq!(solution.statistic.cost, 460., "the premium must be reported in the cost: {solution:?}");
    assert_eq!(solution.statistic.times.off_hours, 4, "the time outside the regular hours must be reported");
    let written = serde_json::to_value(&solution.statistic.times).expect("times serialize");
    assert_eq!(written["offHours"], 4, "the solution format is camelCase: {written}");
}

#[test]
fn reports_nothing_outside_without_regular_hours() {
    let solution = solve(None, None);

    assert_eq!(solution.tours.len(), 1);
    assert_eq!(solution.statistic.cost, 426.);
    assert_eq!(solution.statistic.times.off_hours, 0);
}
