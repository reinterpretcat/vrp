use crate::format::problem::*;
use crate::generator::*;
use crate::helpers::solve_with_metaheuristic_and_iterations;
use proptest::prelude::*;

fn get_jobs_groups() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), Just(Some("one".to_string())), Just(None), Just(Some("two".to_string()))]
}

fn job_prototype() -> impl Strategy<Value = Job> {
    delivery_job_prototype(
        job_task_prototype(
            job_place_prototype(
                generate_location(&DEFAULT_BOUNDING_BOX),
                generate_durations(1..10),
                generate_no_time_windows(),
                generate_no_tags(),
            ),
            generate_simple_demand(1..5),
            generate_no_order(),
        ),
        generate_no_jobs_skills(),
        generate_no_jobs_value(),
        get_jobs_groups(),
    )
}

prop_compose! {
    fn get_problem_with_groups()
    (
     plan in generate_plan(generate_jobs(job_prototype(), 1..512)),
     fleet in generate_fleet(
        generate_vehicles(default_vehicle_type_prototype(), 1..4),
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
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    #[ignore]
    fn can_solve_problem_with_groups(problem in get_problem_with_groups()) {
        solve_with_metaheuristic_and_iterations(problem, None, 10);
    }
}
