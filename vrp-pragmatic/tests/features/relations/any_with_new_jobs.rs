use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

fn create_and_solve_problem_with_three_jobs(any_relation_jobs: Vec<String>) -> Solution {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
            ],
            relations: Some(vec![Relation {
                type_field: RelationType::Any,
                jobs: any_relation_jobs,
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_open_vehicle_shift()],
                capacity: vec![3],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    solve_with_metaheuristic(problem, Some(vec![matrix]))
}

#[test]
fn can_use_any_relation_with_new_job_for_one_vehicle_with_open_end() {
    let solution = create_and_solve_problem_with_three_jobs(to_strings(vec!["job1", "job3"]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![3])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(1., 2.)
                            .load(vec![2])
                            .distance(1)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(3., 4.)
                            .load(vec![1])
                            .distance(2)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((3., 0.))
                            .schedule_stamp(5., 6.)
                            .load(vec![0])
                            .distance(3)
                            .build_single("job3", "delivery"),
                    ])
                    .statistic(StatisticBuilder::default().driving(3).serving(3).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_reshuffle_jobs_in_more_effective_order_than_specified_by_any() {
    let solution = create_and_solve_problem_with_three_jobs(to_strings(vec!["job3", "job1", "job2"]));

    assert_eq!(solution.tours.len(), 1);
    assert_eq!(get_ids_from_tour(solution.tours.first().unwrap()), vec![["departure"], ["job1"], ["job2"], ["job3"]]);
}
