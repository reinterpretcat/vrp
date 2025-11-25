use crate::format::problem::*;
use crate::helpers::*;

fn build_problem(objectives: Option<Vec<Objective>>, min_shifts: Option<VehicleMinShifts>) -> (Problem, Matrix) {
    let jobs = vec![
        create_delivery_job("job1", (1., 0.)),
        create_delivery_job("job2", (2., 0.)),
        create_delivery_job("job3", (3., 0.)),
    ];

    let fleet = Fleet {
        vehicles: vec![VehicleType {
            type_id: "vehicle_type".to_string(),
            vehicle_ids: vec!["vehicle_1".to_string(), "vehicle_2".to_string()],
            profile: create_default_vehicle_profile(),
            costs: VehicleCosts { fixed: Some(0.), distance: 1., time: 1. },
            shifts: vec![create_default_vehicle_shift()],
            capacity: vec![10],
            skills: None,
            limits: None,
            min_shifts,
        }],
        profiles: create_default_matrix_profiles(),
        resources: None,
    };

    let mut problem = create_empty_problem();
    problem.plan = Plan { jobs, relations: None, clustering: None };
    problem.fleet = fleet;
    problem.objectives = objectives;

    let matrix = create_matrix_from_problem(&problem);

    (problem, matrix)
}

#[test]
fn min_vehicle_shifts_constraint_enforces_usage() {
    let (problem_without_requirement, matrix) = build_problem(Some(vec![Objective::MinimizeCost]), None);
    let (problem_with_requirement, _) = build_problem(
        Some(vec![Objective::MinimizeCost]),
        Some(VehicleMinShifts { value: 1, allow_zero_usage: false }),
    );
    let matrices = vec![matrix];

    let solution_without_requirement =
        solve_with_cheapest_insertion(problem_without_requirement, Some(matrices.clone()));
    let solution_with_requirement = solve_with_cheapest_insertion(problem_with_requirement, Some(matrices));

    assert_eq!(solution_without_requirement.tours.len(), 1);
    assert_eq!(solution_with_requirement.tours.len(), 2);
}
