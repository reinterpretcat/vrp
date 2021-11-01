use crate::format::problem::*;
use crate::helpers::*;
use hashbrown::HashSet;

#[test]
fn can_group_jobs() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_group("job1", vec![1., 0.], "one"),
                create_delivery_job("job2", vec![2., 0.]),
                create_delivery_job_with_group("job3", vec![9., 0.], "one"),
                create_delivery_job("job4", vec![8., 0.]),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![
                VehicleType {
                    type_id: "type1".to_string(),
                    vehicle_ids: vec!["type1_1".to_string()],
                    shifts: vec![create_default_vehicle_shift_with_locations((0., 0.), (0., 0.))],
                    capacity: vec![2],
                    ..create_default_vehicle_type()
                },
                VehicleType {
                    type_id: "type2".to_string(),
                    vehicle_ids: vec!["type2_1".to_string()],
                    shifts: vec![create_default_vehicle_shift_with_locations((10., 0.), (10., 0.))],
                    capacity: vec![2],
                    ..create_default_vehicle_type()
                },
            ],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.tours.iter().any(|tour| {
        tour.stops
            .iter()
            .flat_map(|stop| stop.activities.iter())
            .map(|activity| activity.job_id.as_str())
            .filter(|id| *id == "job1" || *id == "job3")
            .collect::<HashSet<_>>()
            .len()
            == 2
    }));
}
