use crate::construction::constraints::conditional::ConditionalJobModule;
use crate::construction::constraints::{ConcreteJobContextTransition, ConstraintModule};
use crate::construction::heuristics::SolutionContext;
use crate::helpers::models::problem::{get_job_id, test_fleet, test_single_with_id};
use crate::models::problem::Job;
use crate::models::solution::Registry;
use hashbrown::HashSet;

fn get_jobs(ids: Vec<&str>) -> Vec<Job> {
    ids.iter().map(|s| Job::Single(test_single_with_id(s))).collect()
}

fn get_ids(jobs: &Vec<Job>) -> Vec<&str> {
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
    let required_set1: HashSet<String> = required_ids.iter().map(|s| s.to_string()).collect();
    let required_set2 = required_set1.clone();

    let mut ctx = SolutionContext {
        required,
        ignored,
        unassigned: Default::default(),
        locked: Default::default(),
        routes: Default::default(),
        registry: Registry::new(&test_fleet()),
    };
    let conditional = ConditionalJobModule::new(Box::new(ConcreteJobContextTransition {
        remove_required: move |_, job| !required_set1.contains(get_job_id(&job)),
        promote_required: move |_, job| required_set2.contains(get_job_id(&job)),
        remove_locked: |_, _| false,
        promote_locked: |_, _| false,
    }));

    conditional.accept_solution_state(&mut ctx);

    assert_eq!(get_ids(&ctx.required), required_ids);
    assert_eq!(get_ids(&ctx.ignored), ignored_ids);
}

#[test]
fn can_promote_locked_jobs() {
    let jobs = get_jobs(vec!["s1", "s2", "s3", "s4", "s5"]);
    let already_locked_jobs: HashSet<String> = vec!["s1", "s2", "s3"].into_iter().map(|s| s.to_string()).collect();
    let expected_ids: Vec<String> = vec!["s1".to_string(), "s2".to_string(), "s5".to_string()];
    let expected_locked_jobs1: HashSet<String> = expected_ids.iter().cloned().collect();
    let expected_locked_jobs2 = expected_locked_jobs1.clone();

    let mut ctx = SolutionContext {
        required: jobs.clone(),
        ignored: Default::default(),
        unassigned: Default::default(),
        locked: jobs.iter().filter(move |job| already_locked_jobs.contains(get_job_id(&job))).cloned().collect(),
        routes: Default::default(),
        registry: Registry::new(&test_fleet()),
    };
    let conditional = ConditionalJobModule::new(Box::new(ConcreteJobContextTransition {
        remove_required: |_, _| false,
        promote_required: |_, _| false,
        remove_locked: move |_, job| !expected_locked_jobs1.contains(get_job_id(&job)),
        promote_locked: move |_, job| expected_locked_jobs2.contains(get_job_id(&job)),
    }));

    conditional.accept_solution_state(&mut ctx);

    let mut result_ids: Vec<String> =
        get_ids(&ctx.locked.iter().cloned().collect()).iter().map(|s| s.to_string()).collect();
    result_ids.sort();
    assert_eq!(result_ids, expected_ids);
}
