use crate::checker::*;
use crate::format::problem::*;
use crate::generator::*;

use proptest::prelude::*;

fn vehicle_type_prototype() -> impl Strategy<Value = VehicleType> {
    generate_vehicle(
        2..4,
        Just("car".to_string()),
        // NOTE must be equal or bigger than total amount jobs in relations
        generate_simple_capacity(150..200),
        default_costs_prototype(),
        generate_no_skills(),
        generate_no_limits(),
        default_vehicle_shifts(),
    )
}

fn relation_job_prototype() -> impl Strategy<Value = Job> {
    delivery_job_prototype(
        job_task_prototype(
            job_place_prototype(
                generate_location(&DEFAULT_BOUNDING_BOX),
                generate_durations(10..20),
                generate_no_time_windows(),
            ),
            generate_simple_demand(1..2),
            generate_no_tags(),
        ),
        generate_no_priority(),
        generate_no_skills(),
    )
}

prop_compose! {
    fn create_problem_with_relations()
    (
    plan  in generate_plan(generate_jobs(relation_job_prototype(), 1..512)),
    fleet in generate_fleet(generate_vehicles(vehicle_type_prototype(), 1..4), default_profiles())
    )
    (
    relations in generate_relations(&plan.jobs, &fleet.vehicles, 1..10, 1..15),
    plan in Just(plan),
    fleet in Just(fleet)
    ) -> Problem {

        assert!(!relations.is_empty());

        Problem {
            plan: Plan {
                relations: Some(relations),
                ..plan
            },
            fleet,
            objectives: None,
            config: None
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    #[ignore]
    fn can_solve_problem_with_relations(problem in create_problem_with_relations()) {
        let result = solve_and_check(problem, None);

        assert_eq!(result, Ok(()));
    }
}
