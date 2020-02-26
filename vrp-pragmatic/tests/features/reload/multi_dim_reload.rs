use crate::format_time;
use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_use_multi_dim_capacity() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_demand("job1", vec![1., 0.], vec![1, 1]),
                create_delivery_job_with_demand("job2", vec![2., 0.], vec![1, 1]),
            ],
            relations: None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    start: VehiclePlace { time: format_time(0.), location: vec![0., 0.].to_loc() },
                    end: Some(VehiclePlace { time: format_time(100.).to_string(), location: vec![0., 0.].to_loc() }),
                    breaks: None,
                    reloads: Some(vec![VehicleReload {
                        times: None,
                        location: vec![0., 0.].to_loc(),
                        duration: 2.0,
                        tag: None,
                    }]),
                }],
                capacity: vec![1, 1],
                amount: 1,
                skills: None,
                limits: None,
            }],
            profiles: create_default_profiles(),
        },
        config: None,
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 26.,
                distance: 6,
                duration: 10,
                times: Timing { driving: 6, serving: 4, waiting: 0, break_time: 0 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity_md(
                        "departure",
                        "departure",
                        (0., 0.),
                        vec![1, 1],
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity_md(
                        "job1",
                        "delivery",
                        (1., 0.),
                        vec![0, 0],
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        1
                    ),
                    create_stop_with_activity_md(
                        "reload",
                        "reload",
                        (0., 0.),
                        vec![1, 1],
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:05Z"),
                        2
                    ),
                    create_stop_with_activity_md(
                        "job2",
                        "delivery",
                        (2., 0.),
                        vec![0, 0],
                        ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                        4
                    ),
                    create_stop_with_activity_md(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        vec![0, 0],
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:10Z"),
                        6
                    ),
                ],
                statistic: Statistic {
                    cost: 26.,
                    distance: 6,
                    duration: 10,
                    times: Timing { driving: 6, serving: 4, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: None,
        }
    );
}
