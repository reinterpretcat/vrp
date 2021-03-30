use super::create_geojson_solution;
use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_create_geo_json() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![1., 0.]),
                create_delivery_job_with_demand("job2", vec![2., 0.], vec![10]),
                create_delivery_job("job3", vec![3., 0.]),
            ],
            relations: None,
        },
        fleet: Fleet { vehicles: vec![create_default_vehicle("my_vehicle")], profiles: create_default_profiles() },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);
    let core_problem = (problem.clone(), vec![matrix.clone()]).read_pragmatic().unwrap();
    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));
    let geo_json = create_geojson_solution(&core_problem, &solution).unwrap();

    assert_eq!(geo_json.features.len(), 6);
}
