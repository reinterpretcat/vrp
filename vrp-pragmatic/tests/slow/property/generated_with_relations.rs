use crate::checker::*;
use crate::generator::*;
use crate::json::problem::*;

use proptest::prelude::*;

pub fn relation_job_prototype() -> impl Strategy<Value = Job> {
    delivery_job_prototype(
        job_task_prototype(
            job_place_prototype(
                generate_simple_locations(1..100),
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
        (plan in generate_plan(generate_jobs(relation_job_prototype(), 1..256)),
         fleet in generate_fleet(generate_vehicles(default_vehicle_type_prototype(), 1..4), default_profiles())
        )
        (relations in generate_relations(&plan.jobs, &fleet.vehicles, 1..10, 2..20), plan in Just(plan), fleet in Just(fleet))
        -> Problem {
        // NOTE prop_filter in original strategy does not work as expected
        let relations = relations.into_iter().filter(|r| !r.jobs.is_empty()).collect();

        Problem {
            plan: Plan {
                relations: Some(relations),
                ..plan
            },
            fleet,
            config: None
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    #[ignore]
    fn can_solve_problem_with_relations(problem in create_problem_with_relations()) {
        let ctx = create_checker_context(problem);

        let result = check_relations(&ctx);

        assert_eq!(result, Ok(()));
    }
}
