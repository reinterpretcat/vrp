use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_use_two_pickup_delivery_jobs_and_relation_with_one_vehicle() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_pickup_delivery_job("job1", (20., 0.), (15., 0.)),
                create_pickup_delivery_job("job2", (5., 0.), (20., 0.)),
            ],
            relations: Some(vec![Relation {
                type_field: RelationType::Sequence,
                jobs: to_strings(vec!["job1", "job2", "job1", "job2"]),
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_vehicle_shift_with_locations((10., 0.), (10., 0.))],
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
                            .coordinate((10., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![0])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((20., 0.))
                            .schedule_stamp(10., 11.)
                            .load(vec![1])
                            .distance(10)
                            .build_single_tag("job1", "pickup", "p1"),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(26., 27.)
                            .load(vec![2])
                            .distance(25)
                            .build_single_tag("job2", "pickup", "p1"),
                        StopBuilder::default()
                            .coordinate((15., 0.))
                            .schedule_stamp(37., 38.)
                            .load(vec![1])
                            .distance(35)
                            .build_single_tag("job1", "delivery", "d1"),
                        StopBuilder::default()
                            .coordinate((20., 0.))
                            .schedule_stamp(43., 44.)
                            .load(vec![0])
                            .distance(40)
                            .build_single_tag("job2", "delivery", "d1"),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(54., 54.)
                            .load(vec![0])
                            .distance(50)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(50).serving(4).build())
                    .build()
            )
            .build()
    );
}
