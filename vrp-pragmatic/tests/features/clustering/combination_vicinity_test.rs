use super::*;

parameterized_test! {can_handle_job_in_relation_with_vicinity_cluster, type_field, {
    can_handle_job_in_relation_with_vicinity_cluster_impl(type_field);
}}

can_handle_job_in_relation_with_vicinity_cluster! {
    case_01_strict: RelationType::Any,
    case_02_sequence: RelationType::Sequence,
    case_03_strict: RelationType::Strict,
}

fn can_handle_job_in_relation_with_vicinity_cluster_impl(type_field: RelationType) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (1., 0.)), create_delivery_job("job2", (1., 0.))],
            clustering: Some(Clustering::Vicinity {
                profile: VehicleProfile { matrix: "car".to_string(), scale: None },
                threshold: VicinityThresholdPolicy {
                    duration: 10.,
                    distance: 10.,
                    min_shared_time: None,
                    smallest_time_window: None,
                    max_jobs_per_cluster: None,
                },
                visiting: VicinityVisitPolicy::Continue,
                serving: VicinityServingPolicy::Original { parking: 300. },
                filtering: None,
            }),
            relations: Some(vec![Relation {
                type_field,
                jobs: vec!["departure".to_string(), "job1".to_string()],
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
            ..create_empty_plan()
        },
        fleet: Fleet { vehicles: vec![create_default_vehicle("my_vehicle")], ..create_default_fleet() },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert_eq!(solution.tours[0].stops.len(), 3);
    assert_eq!(solution.tours[0].stops[1].activities().len(), 2);
}
