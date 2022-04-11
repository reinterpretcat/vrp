use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

parameterized_test! {can_use_vehicle_with_two_tours_and_two_jobs, (jobs, unassigned), {
    can_use_vehicle_with_two_tours_and_two_jobs_impl(jobs, unassigned);
}}

can_use_vehicle_with_two_tours_and_two_jobs! {
    case01: (vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.))],
            None),
    case02: (vec![
               create_delivery_job("job1", (1., 0.)),
               create_delivery_job("job2", (2., 0.)),
               create_delivery_job("job3", (3., 0.))
             ],
             Some(vec![
               UnassignedJob {
                    job_id: "job3".to_string(),
                    reasons: vec![UnassignedJobReason {
                        code: "CAPACITY_CONSTRAINT".to_string(),
                        description: "does not fit into any vehicle due to capacity".to_string()
                    }]
                }
             ])),
}

fn can_use_vehicle_with_two_tours_and_two_jobs_impl(jobs: Vec<Job>, unassigned: Option<Vec<UnassignedJob>>) {
    let problem = Problem {
        plan: Plan { jobs, ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: format_time(100.).to_string(),
                        location: (0., 0.).to_loc(),
                    }),
                    dispatch: None,
                    breaks: None,
                    reloads: Some(vec![VehicleReload {
                        times: None,
                        location: (0., 0.).to_loc(),
                        duration: 2.0,
                        tag: None,
                    }]),
                }],
                capacity: vec![1],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 26.,
                distance: 6,
                duration: 10,
                times: Timing { driving: 6, serving: 4, ..Timing::default() },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        1,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        1
                    ),
                    create_stop_with_activity(
                        "reload",
                        "reload",
                        (0., 0.),
                        1,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:05Z"),
                        2
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (2., 0.),
                        0,
                        ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                        4
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:10Z"),
                        6
                    ),
                ],
                statistic: Statistic {
                    cost: 26.,
                    distance: 6,
                    duration: 10,
                    times: Timing { driving: 6, serving: 4, ..Timing::default() },
                },
            }],
            unassigned,
            ..create_empty_solution()
        }
    );
}
