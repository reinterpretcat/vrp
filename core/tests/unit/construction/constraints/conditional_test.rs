use crate::construction::constraints::conditional::ConditionalJobModule;
use crate::construction::constraints::ConstraintModule;
use crate::construction::states::SolutionContext;
use crate::helpers::models::problem::{get_job_id, test_fleet, test_single_job_with_id};
use crate::models::problem::Job;
use crate::models::solution::Registry;
use std::collections::HashSet;
use std::sync::Arc;

fn get_jobs(ids: Vec<&str>) -> Vec<Arc<Job>> {
    ids.iter().map(|s| Arc::new(test_single_job_with_id(s))).collect()
}

fn get_ids(jobs: &Vec<Arc<Job>>) -> Vec<&str> {
    let mut ids: Vec<&str> = jobs.iter().map(|job| get_job_id(job).as_str()).collect();
    ids.sort();
    ids
}

parameterized_test! {can_promote_jobs_between_required_and_ignored, (required, ignored, required_ids, ignored_ids), {
    can_promote_jobs_between_required_and_ignored_impl(required, ignored, required_ids, ignored_ids);
}}

can_promote_jobs_between_required_and_ignored! {
    case01: (vec!["s1", "s2", "s3"], vec!["s4"], vec!["s1"], vec!["s2", "s3", "s4"]),
    case02: (vec!["s1", "s2", "s3"], vec![], vec!["s1", "s2", "s3"], vec![]),
    case03: (vec![], vec!["s1", "s2", "s3"], vec!["s1", "s2", "s3"], vec![]),
}

fn can_promote_jobs_between_required_and_ignored_impl(
    required: Vec<&str>,
    ignored: Vec<&str>,
    required_ids: Vec<&str>,
    ignored_ids: Vec<&str>,
) {
    let required = get_jobs(required);
    let ignored = get_jobs(ignored);
    let required_set: HashSet<String> = required_ids.iter().map(|s| s.to_string()).collect();

    let mut ctx = SolutionContext {
        required,
        ignored,
        unassigned: Default::default(),
        locked: Default::default(),
        routes: Default::default(),
        registry: Registry::new(&test_fleet()),
    };
    let conditional = ConditionalJobModule::new(Box::new(move |_, job| required_set.contains(get_job_id(&job))));

    conditional.accept_solution_state(&mut ctx);

    assert_eq!(get_ids(&ctx.required), required_ids);
    assert_eq!(get_ids(&ctx.ignored), ignored_ids);
}
