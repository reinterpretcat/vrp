use crate::checker::solve_and_check;
use crate::generator::*;
use crate::json::problem::*;

use proptest::prelude::*;

fn simple_job_prototype() -> impl Strategy<Value = JobVariant> {
    prop_oneof![default_delivery_prototype(), default_pickup_prototype()]
}

prop_compose! {
    fn create_problem_with_relations()
        (plan in generate_plan(generate_jobs(simple_job_prototype(), 1..256)),
         fleet in generate_fleet(generate_vehicles(default_vehicle_type_prototype(), 1..4), default_profiles())
        )
        (relations in generate_relations(&plan.jobs, &fleet.types, 1..10, 2..20), plan in Just(plan), fleet in Just(fleet))
        -> Problem {
        // NOTE prop_filter in original strategy does not work as expected
        let relations = relations.into_iter().filter(|r| !r.jobs.is_empty()).collect();

        Problem {
            id: "generated_problem_with_relations".to_string(),
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
        let result = solve_and_check(problem);

        assert_eq!(result, Ok(()));
    }
}
