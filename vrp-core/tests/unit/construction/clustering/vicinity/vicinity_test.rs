use super::*;
use crate::helpers::construction::clustering::vicinity::create_constraint_pipeline;
use crate::helpers::models::domain::*;
use crate::helpers::models::problem::*;

#[test]
fn can_get_check_insertion() {
    let disallow_merge_list = vec!["job2"];
    let jobs = vec![Job::Single(test_single_with_id("job1")), Job::Single(test_single_with_id("job2"))];
    let constraint = create_constraint_pipeline(disallow_merge_list);
    let fleet = test_fleet();
    let problem = create_problem_with_constraint_jobs_and_fleet(constraint, jobs.clone(), fleet);
    let insertion_ctx = InsertionContext { problem, ..create_empty_insertion_context() };
    let actor_filter = |_: &Actor| true;

    let check_insertion = get_check_insertion_fn(insertion_ctx, &actor_filter);

    assert_eq!(check_insertion(jobs.get(0).unwrap()), Ok(()));
    assert_eq!(check_insertion(jobs.get(1).unwrap()), Err(1));
}
