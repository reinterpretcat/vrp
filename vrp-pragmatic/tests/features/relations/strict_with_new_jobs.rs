use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_use_two_strict_relations_with_two_vehicles_with_new_jobs() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
                create_delivery_job("job4", (4., 0.)),
                create_delivery_job("job5", (5., 0.)),
                create_delivery_job("job6", (6., 0.)),
                create_delivery_job("job7", (7., 0.)),
                create_delivery_job("job8", (8., 0.)),
                create_delivery_job("job9", (9., 0.)),
                create_delivery_job("job10", (10., 0.)),
            ],
            relations: Some(vec![
                Relation {
                    type_field: RelationType::Strict,
                    jobs: to_strings(vec!["departure", "job1", "job6", "job4", "job8"]),
                    vehicle_id: "my_vehicle_1".to_string(),
                    shift_index: None,
                },
                Relation {
                    type_field: RelationType::Strict,
                    jobs: to_strings(vec!["departure", "job2", "job3", "job5", "job7"]),
                    vehicle_id: "my_vehicle_2".to_string(),
                    shift_index: None,
                },
            ]),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                capacity: vec![5],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none(), "every job must be served");
    assert_eq!(solution.tours.len(), 2);

    // What the relations claim: each vehicle serves its listed jobs, in the listed order, ahead of
    // anything else it picks up.
    let first = served_job_ids(tour_of(&solution, "my_vehicle_1"));
    let second = served_job_ids(tour_of(&solution, "my_vehicle_2"));

    assert_eq!(first[..4], ["job1", "job6", "job4", "job8"].map(String::from));
    assert_eq!(second[..4], ["job2", "job3", "job5", "job7"].map(String::from));

    // `job9` and `job10` are in no relation, and with a capacity of 5 against four related jobs
    // each vehicle takes exactly one of them. Which one is a tie the solver may break either way —
    // the two are one unit apart, so the total is the same and only that total is asserted.
    assert_eq!(first.len(), 5);
    assert_eq!(second.len(), 5);

    let mut free = vec![first[4].clone(), second[4].clone()];
    free.sort();
    assert_eq!(free, vec!["job10".to_string(), "job9".to_string()]);

    assert_eq!(solution.statistic.distance, 42);
    assert_eq!(solution.statistic.duration, 52);
    assert_eq!(solution.statistic.cost, 114.);
}
