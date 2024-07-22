use crate::format::problem::*;
use crate::generator::*;
use crate::helpers::solve_with_metaheuristic_and_iterations;
use proptest::prelude::*;
use std::ops::Range;
use vrp_core::models::common::Distance;

prop_compose! {
    pub fn get_max_distances(range: Range<i32>)(distance in range) -> Distance {
        distance as Distance
    }
}

fn get_recharge_stations() -> impl Strategy<Value = Vec<VehicleRechargeStation>> {
    prop::collection::vec(
        generate_recharge_station(
            generate_location(&DEFAULT_BOUNDING_BOX),
            generate_durations(300..3600),
            generate_no_tags(),
            generate_no_time_windows(),
        ),
        5..20,
    )
}

prop_compose! {
    fn get_vehicle_type_with_recharges()
    (
     vehicle in default_vehicle_type_prototype(),
     max_distance in get_max_distances(3000..30000),
     stations in get_recharge_stations()
    ) -> VehicleType {
        assert_eq!(vehicle.shifts.len(), 1);

        let mut vehicle = vehicle;

        // set capacity to high and have only one vehicle of such type to have a higher probability
        // for recharge to be kicked in
        vehicle.capacity = vec![10000];
        vehicle.vehicle_ids = vec![format!("{}_1", vehicle.type_id)];

        vehicle.shifts.first_mut().unwrap().end = None;
        vehicle.shifts.first_mut().unwrap().recharges = Some(VehicleRecharges { max_distance, stations });

        vehicle
    }
}

pub fn get_delivery_prototype() -> impl Strategy<Value = Job> {
    delivery_job_prototype(
        job_task_prototype(
            job_place_prototype(
                generate_location(&DEFAULT_BOUNDING_BOX),
                generate_durations(1..10),
                generate_no_time_windows(),
                generate_no_tags(),
            ),
            generate_simple_demand(1..2),
            generate_no_order(),
        ),
        generate_no_jobs_skills(),
        generate_no_jobs_value(),
        generate_no_jobs_group(),
        generate_no_jobs_compatibility(),
    )
}

prop_compose! {
    fn create_problem_with_recharges()
    (
      plan in generate_plan(generate_jobs(get_delivery_prototype(), 1..512)),
      fleet in generate_fleet(
        generate_vehicles(get_vehicle_type_with_recharges(), 1..2),
        default_matrix_profiles())
    ) -> Problem {
        Problem {
            plan,
            fleet,
            objectives: None,
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    #[ignore]
    fn can_solve_problem_with_recharge(problem in create_problem_with_recharges()) {
        solve_with_metaheuristic_and_iterations(problem, None, 10);
    }
}
