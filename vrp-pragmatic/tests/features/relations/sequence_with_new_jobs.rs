use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_use_sequence_relation_with_strict_time_windows() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (10., 0.), vec![(150, 170)], 10.),
                create_delivery_job_with_times("job2", (20., 0.), vec![(20, 30)], 10.),
                create_delivery_job_with_times("job3", (30., 0.), vec![(40, 50)], 10.),
                create_delivery_job_with_times("job4", (40., 0.), vec![(60, 150)], 10.),
                create_delivery_job_with_times("job5", (50., 0.), vec![(70, 80)], 10.),
            ],
            relations: Some(vec![Relation {
                type_field: RelationType::Sequence,
                jobs: to_strings(vec!["job5", "job4"]),
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["my_vehicle_1".to_string()],
                capacity: vec![10],
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
                            .load(vec![5])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((20., 0.))
                            .schedule_stamp(30., 40.)
                            .load(vec![4])
                            .distance(20)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((30., 0.))
                            .schedule_stamp(50., 60.)
                            .load(vec![3])
                            .distance(30)
                            .build_single("job3", "delivery"),
                        StopBuilder::default()
                            .coordinate((50., 0.))
                            .schedule_stamp(80., 90.)
                            .load(vec![2])
                            .distance(50)
                            .build_single("job5", "delivery"),
                        StopBuilder::default()
                            .coordinate((40., 0.))
                            .schedule_stamp(100., 110.)
                            .load(vec![1])
                            .distance(60)
                            .build_single("job4", "delivery"),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(140., 160.)
                            .load(vec![0])
                            .distance(90)
                            .build_single_time("job1", "delivery", (150., 160.)),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(170., 17.)
                            .load(vec![0])
                            .distance(100)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(100).serving(50).waiting(10).build())
                    .build()
            )
            .build()
    );
}
