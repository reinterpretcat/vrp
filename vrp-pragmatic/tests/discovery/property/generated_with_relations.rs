use crate::format::problem::*;
use crate::generator::*;
use crate::helpers::solve_with_metaheuristic_and_iterations;

use proptest::prelude::*;

fn vehicle_type_prototype() -> impl Strategy<Value = VehicleType> {
    generate_vehicle(
        2..4,
        default_vehicle_profile(),
        // NOTE must be equal or bigger than total amount jobs in relations
        generate_simple_capacity(150..200),
        default_costs_prototype(),
        generate_no_vehicle_skills(),
        generate_no_limits(),
        generate_shifts(
            generate_shift(
                generate_location(&DEFAULT_BOUNDING_BOX).prop_flat_map(|location| {
                    Just((
                        ShiftStart { earliest: default_time_plus_offset(9), latest: None, location: location.clone() },
                        None,
                    ))
                }),
                generate_no_dispatch(),
                default_breaks_prototype(),
                generate_no_reloads(),
            ),
            1..2,
        ),
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
        generate_no_order(),
        generate_no_jobs_skills(),
        generate_no_jobs_value(),
    )
}

prop_compose! {
    fn create_problem_with_relations()
    (
    plan  in generate_plan(generate_jobs(relation_job_prototype(), 1..512)),
    fleet in generate_fleet(
        generate_vehicles(vehicle_type_prototype(), 1..4),
        default_matrix_profiles())
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
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    #[ignore]
    fn can_solve_problem_with_relations(problem in create_problem_with_relations()) {
        solve_with_metaheuristic_and_iterations(problem, None, 10);
    }
}
