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
                        description: "does not fit into any vehicle due to capacity".to_string(),
                        details: Some(vec![UnassignedJobDetail { vehicle_id: "my_vehicle_1".to_string(), shift_index: 0 }]),
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
                    start: ShiftStart { earliest: format_time(0), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(100), location: (0., 0.).to_loc() }),
                    breaks: None,
                    reloads: Some(vec![VehicleReload {
                        location: (0., 0.).to_loc(),
                        duration: 2,
                        ..create_default_reload()
                    }]),
                    recharges: None,
                }],
                capacity: vec![1],
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
                            .schedule_stamp(0, 0)
                            .load(vec![1])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(1, 2)
                            .load(vec![0])
                            .distance(1)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(3, 5)
                            .load(vec![1])
                            .distance(2)
                            .build_single("reload", "reload"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(7, 8)
                            .load(vec![0])
                            .distance(4)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(10, 10)
                            .load(vec![0])
                            .distance(6)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(6).serving(4).build())
                    .build()
            )
            .unassigned(unassigned)
            .build()
    );
}
