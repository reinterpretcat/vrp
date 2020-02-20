use crate::checker::solve_and_check;
use crate::generator::*;
use crate::json::problem::*;

use proptest::prelude::*;

fn get_reloads() -> impl Strategy<Value = Option<Vec<VehicleReload>>> {
    prop::collection::vec(
        generate_reload(
            Just(default_vehicle_location()),
            generate_durations(300..3600),
            generate_no_tags(),
            default_job_single_day_time_windows().prop_map(|tws| Some(tws)),
        ),
        1..4,
    )
    .prop_map(|reloads| Some(reloads))
}

prop_compose! {
    fn get_vehicle_type_with_reloads()
        (vehicle in default_vehicle_type_prototype(),
         reloads in get_reloads()
        ) -> VehicleType {
        assert_eq!(vehicle.shifts.len(), 1);

        let mut vehicle = vehicle;
        vehicle.shifts.first_mut().unwrap().reloads = reloads;

        vehicle
    }
}

prop_compose! {
    fn create_problem_with_reloads()
        (plan in generate_plan(generate_jobs(default_job_prototype(), 1..256)),
         fleet in generate_fleet(generate_vehicles(get_vehicle_type_with_reloads(), 1..4), default_profiles())
        )
        -> Problem {
        Problem {
            id: "generated_problem_with_reloads".to_string(),
            plan,
            fleet,
            config: None
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    #[ignore]
    fn can_solve_problem_with_reloads(problem in create_problem_with_reloads()) {
        let result = solve_and_check(problem);

        assert_eq!(result, Ok(()));
    }
}
