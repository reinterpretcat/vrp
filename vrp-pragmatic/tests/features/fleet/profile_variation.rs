use crate::format::problem::*;
use crate::helpers::*;

fn create_vehicle_type(type_id: &str, scale: Option<f64>) -> VehicleType {
    VehicleType {
        type_id: type_id.to_string(),
        profile: VehicleProfile { matrix: "car".to_string(), scale },
        vehicle_ids: vec![format!("{}_1", type_id)],
        ..create_default_vehicle_type()
    }
}
#[test]
fn can_use_scale() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", vec![10., 0.])], ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![create_vehicle_type("normal", None), create_vehicle_type("slow", Some(0.5))],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert_eq!(solution.tours.len(), 1);
    let tour = solution.tours.first().unwrap();
    assert_eq!(tour.vehicle_id, "slow_1");
    assert_eq!(tour.statistic.distance, 20);
    assert_eq!(tour.statistic.duration, 11)
}
