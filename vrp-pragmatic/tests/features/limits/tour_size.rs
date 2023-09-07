use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_skip_job_from_multiple_because_of_tour_size() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_open_vehicle_shift()],
                limits: Some(VehicleLimits { max_distance: None, max_duration: None, tour_size: Some(2) }),
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![2])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(1., 2.)
                            .load(vec![1])
                            .distance(1)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(3., 4.)
                            .load(vec![0])
                            .distance(2)
                            .build_single("job2", "delivery")
                    ])
                    .statistic(StatisticBuilder::default().driving(2).serving(2).break_time(2).build())
                    .build()
            )
            .unassigned(Some(vec![UnassignedJob {
                job_id: "job3".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: "TOUR_SIZE_CONSTRAINT".to_string(),
                    description: "cannot be assigned due to tour size constraint of vehicle".to_string(),
                    details: Some(vec![UnassignedJobDetail { vehicle_id: "my_vehicle_1".to_string(), shift_index: 0 }]),
                }]
            }]))
            .build()
    );
}
