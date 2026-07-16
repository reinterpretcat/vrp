use crate::format::problem::Objective::*;
use crate::format::problem::*;
use crate::format::solution::Tour;
use crate::format_time;
use crate::helpers::*;
use std::collections::HashMap;
use vrp_core::prelude::Float;

fn get_activities_count(tour: &Tour) -> usize {
    tour.stops
        .iter()
        .map(|stop| stop.activities().iter().filter(|activity| activity.activity_type == "delivery").count())
        .sum()
}

/// Like `create_default_vehicle_shift_with_locations`, but starts after `offset` so that it does
/// not overlap with a shift ending at t=1000 (used to give a vehicle a second, later shift).
fn create_later_vehicle_shift_with_locations(offset: Float, start: (f64, f64), end: (f64, f64)) -> VehicleShift {
    VehicleShift {
        start: ShiftStart { earliest: format_time(offset), latest: None, location: start.to_loc() },
        end: Some(ShiftEnd { earliest: None, latest: format_time(offset + 1000.), location: end.to_loc() }),
        breaks: None,
        reloads: None,
        recharges: None,
        job_times: None,
    }
}

#[test]
fn can_balance_period_by_production_value() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_production_value("job1.0", (1., 0.), 10.),
                create_delivery_job_with_production_value("job1.1", (1., 0.), 10.),
                create_delivery_job_with_production_value("job1.2", (1., 0.), 10.),
                create_delivery_job_with_production_value("job1.3", (1., 0.), 10.),
                create_delivery_job_with_production_value("job2.0", (2., 0.), 10.),
                create_delivery_job_with_production_value("job2.1", (2., 0.), 10.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![
                VehicleType {
                    vehicle_ids: vec!["my_vehicle1".to_string()],
                    shifts: vec![create_default_open_vehicle_shift()],
                    capacity: vec![4],
                    ..create_default_vehicle_type()
                },
                VehicleType {
                    type_id: "my_vehicle2".to_string(),
                    vehicle_ids: vec!["my_vehicle2".to_string()],
                    shifts: vec![create_default_vehicle_shift_with_locations((3., 0.), (3., 0.))],
                    capacity: vec![4],
                    ..create_default_vehicle_type()
                },
            ],
            ..create_default_fleet()
        },
        objectives: Some(vec![
            MinimizeUnassigned { breaks: None },
            BalancePeriod { metric: BalancePeriodMetric::ProductionValue },
            MinimizeCost,
        ]),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // Two employees with equal shift capacity (1 each) and equal per-job production value =>
    // a period-balanced solution splits 3/3 (30 production value per employee).
    assert_eq!(solution.tours.len(), 2);
    assert_eq!(solution.tours.iter().map(get_activities_count).min().unwrap(), 3);
    assert_eq!(solution.tours.iter().map(get_activities_count).max().unwrap(), 3);
}

#[test]
fn can_balance_period_by_duration() {
    // Two employees (my_vehicle_1, my_vehicle_2), each with a single shift (shift capacity 1),
    // so balance-period's per-employee ratio here reduces to the same math as the per-tour
    // balance-duration objective (division by a constant capacity of 1) -- same shape as the
    // proven-stable `can_balance_duration` test in `balance_transport.rs`, just scoped through
    // balance-period's goal-reader arm instead. This exercises the goal-reader's Duration arm and
    // its read of the TransportState-maintained route totals (`get_total_duration`) end-to-end
    // through the actual solver, not just deserialization.
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_duration("job1", (1., 0.), 10.),
                create_delivery_job_with_duration("job2", (2., 0.), 10.),
                create_delivery_job_with_duration("job3", (3., 0.), 10.),
                create_delivery_job_with_duration("job4", (4., 0.), 10.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                shifts: vec![create_default_open_vehicle_shift()],
                capacity: vec![3],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives: Some(vec![
            MinimizeUnassigned { breaks: None },
            BalancePeriod { metric: BalancePeriodMetric::Duration },
            MinimizeCost,
        ]),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.tours.len(), 2);
    assert!(solution.tours.first().unwrap().statistic.duration < 30);
    assert!(solution.tours.last().unwrap().statistic.duration < 30);
}

#[test]
fn can_balance_period_normalized_by_shift_capacity() {
    // my_vehicle2 has twice the shift capacity (2 shifts) of my_vehicle1 (1 shift), so a
    // period-balanced solution should give my_vehicle2 twice the total load of my_vehicle1
    // across the whole period, i.e. a 2/4 split of the 6 jobs (used/capacity ratio equal:
    // 2/1 == 4/2). This is asserted per employee (vehicle_id) summed across all of that
    // employee's tours, since the objective is agnostic to how an employee's period load is
    // distributed across their own shifts/tours (e.g. my_vehicle2's 4 jobs may land in a single
    // tour, or be split across both of its shifts).
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1.0", (1., 0.)),
                create_delivery_job("job1.1", (1., 0.)),
                create_delivery_job("job1.2", (1., 0.)),
                create_delivery_job("job1.3", (1., 0.)),
                create_delivery_job("job2.0", (2., 0.)),
                create_delivery_job("job2.1", (2., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![
                VehicleType {
                    vehicle_ids: vec!["my_vehicle1".to_string()],
                    shifts: vec![create_default_open_vehicle_shift()],
                    capacity: vec![6],
                    ..create_default_vehicle_type()
                },
                VehicleType {
                    type_id: "my_vehicle2".to_string(),
                    vehicle_ids: vec!["my_vehicle2".to_string()],
                    shifts: vec![
                        create_default_vehicle_shift_with_locations((3., 0.), (3., 0.)),
                        create_later_vehicle_shift_with_locations(2000., (3., 0.), (3., 0.)),
                    ],
                    capacity: vec![6],
                    ..create_default_vehicle_type()
                },
            ],
            ..create_default_fleet()
        },
        objectives: Some(vec![
            MinimizeUnassigned { breaks: None },
            BalancePeriod { metric: BalancePeriodMetric::Activities },
            MinimizeCost,
        ]),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    let totals = solution.tours.iter().fold(HashMap::<String, usize>::new(), |mut acc, tour| {
        *acc.entry(tour.vehicle_id.clone()).or_insert(0) += get_activities_count(tour);
        acc
    });

    assert_eq!(totals.get("my_vehicle1").copied().unwrap_or(0), 2);
    assert_eq!(totals.get("my_vehicle2").copied().unwrap_or(0), 4);
}
