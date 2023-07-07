use crate::format::problem::Objective::*;
use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_compact_tour() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job-6", (-6., 0.)),
                create_delivery_job("job-5", (-5., 0.)),
                create_delivery_job("job-4", (-4., 0.)),
                create_delivery_job("job-3", (-3., 0.)),
                create_delivery_job("job-2", (-2., 0.)),
                create_delivery_job("job-1", (-1., 0.)),
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
                create_delivery_job("job4", (4., 0.)),
                create_delivery_job("job5", (5., 0.)),
                create_delivery_job("job6", (6., 0.)),
            ],
            relations: Some(vec![Relation {
                type_field: RelationType::Any,
                jobs: vec!["job-4".to_string(), "job4".to_string()],
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                shifts: vec![create_default_open_vehicle_shift()],
                ..create_vehicle_with_capacity("my_vehicle", vec![6])
            }],
            ..create_default_fleet()
        },
        objectives: Some(vec![
            vec![MinimizeUnassignedJobs { breaks: None }],
            vec![MinimizeTours],
            vec![CompactTour { options: CompactOptions { job_radius: 2, threshold: 2, distance: 0. } }],
            vec![MinimizeCost],
        ]),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), 1000);

    [("my_vehicle_1", ["-4", "-5", "-6", "4", "5", "6"]), ("my_vehicle_2", ["-1", "-2", "-3", "1", "2", "3"])]
        .into_iter()
        .for_each(|(vehicle_id, expected_job_ids)| {
            let expected_job_ids =
                expected_job_ids.into_iter().map(|number: &str| format!("job{number}")).collect::<Vec<_>>();
            let actual_ids =
                get_ids_from_tour_sorted(solution.tours.iter().find(|tour| tour.vehicle_id == vehicle_id).unwrap())
                    .into_iter()
                    .flatten()
                    .filter(|id| id != "arrival" && id != "departure")
                    .collect::<Vec<_>>();

            assert_eq!(actual_ids, expected_job_ids);
        })
}
