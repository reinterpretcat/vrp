use super::*;
use crate::helpers::construction::clustering::vicinity::*;
use crate::helpers::models::domain::*;
use crate::helpers::models::problem::*;

#[test]
fn can_get_check_insertion() {
    let disallow_merge_list = vec!["job2"];
    let jobs = vec![
        SingleBuilder::default().id("job1").build_as_job_ref(),
        SingleBuilder::default().id("job2").build_as_job_ref(),
    ];
    let constraint = create_goal_context(disallow_merge_list);
    let fleet = test_fleet();
    let problem = create_problem_with_goal_ctx_jobs_and_fleet(constraint, jobs.clone(), fleet);
    let insertion_ctx = InsertionContext { problem, ..create_empty_insertion_context() };
    let actor_filter = Arc::new(|_: &Actor| true);

    let check_insertion = get_check_insertion_fn(insertion_ctx, actor_filter);

    assert_eq!(check_insertion(jobs.get(0).unwrap()), Ok(()));
    assert_eq!(check_insertion(jobs.get(1).unwrap()), Err(1));
}

#[test]
pub fn can_create_job_clusters() {
    let jobs = vec![
        SingleBuilder::default().id("job1").build_as_job_ref(),
        SingleBuilder::default().id("job2").build_as_job_ref(),
        SingleBuilder::default().id("job3").build_as_job_ref(),
    ];
    let constraint = create_goal_context(vec![]);
    let filtering =
        FilterPolicy { job_filter: Arc::new(|job| get_job_id(job) != "job3"), actor_filter: Arc::new(|_| true) };
    let config = ClusterConfig { filtering, ..create_cluster_config() };
    let fleet = test_fleet();
    let problem = create_problem_with_goal_ctx_jobs_and_fleet(constraint, jobs, fleet);

    let clusters = create_job_clusters(problem, Arc::new(Environment::default()), &config);

    assert_eq!(clusters.len(), 1);
    let cluster = clusters.first().unwrap();
    let clustered = &cluster.1;
    assert_eq!(clustered.len(), 2);
}
