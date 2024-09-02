use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_wait_for_job_start() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_skills(
                "job1",
                (1., 0.),
                all_of_skills(vec!["unique_skill".to_string()]),
            )],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![
                create_default_vehicle("vehicle_without_skill"),
                VehicleType {
                    type_id: "vehicle_with_skill".to_string(),
                    vehicle_ids: vec!["vehicle_with_skill_1".to_string()],
                    shifts: vec![create_default_vehicle_shift_with_locations((10., 0.), (10., 0.))],
                    skills: Some(vec!["unique_skill".to_string()]),
                    ..create_default_vehicle_type()
                },
            ],
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
                    .type_id("vehicle_with_skill")
                    .vehicle_id("vehicle_with_skill_1")
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(0, 0)
                            .load(vec![1])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(9, 10)
                            .load(vec![0])
                            .distance(9)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(19, 19)
                            .load(vec![0])
                            .distance(18)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(18).serving(1).build())
                    .build()
            )
            .build()
    );
}
