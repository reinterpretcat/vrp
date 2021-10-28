use super::*;
use crate::helpers::construction::clustering::vicinity::{create_cluster_config, create_constraint_pipeline};
use crate::helpers::models::domain::create_problem_with_constraint_jobs_and_fleet;
use crate::helpers::models::problem::*;
use crate::models::problem::Job;

fn create_test_jobs() -> Vec<Job> {
    vec![
        Job::Single(test_single_with_id_and_location("job1", Some(1))),
        Job::Single(test_single_with_id_and_location("job2", Some(2))),
        Job::Single(test_single_with_id_and_location("job3", Some(3))),
        Job::Single(test_single_with_id_and_location("job4_outlier", Some(20))),
    ]
}

#[test]
pub fn can_create_problem_with_clusters() {
    let problem_jobs = create_test_jobs();
    let constraint = create_constraint_pipeline(vec![]);
    let environment = Arc::new(Environment::default());
    let problem = create_problem_with_constraint_jobs_and_fleet(constraint, problem_jobs.clone(), test_fleet());

    let problem = VicinityClustering::new(create_cluster_config()).pre_process(problem, environment);

    let jobs = problem.jobs.all().collect::<Vec<_>>();
    assert_eq!(jobs.len(), 2);
    assert!(jobs.iter().find(|job| get_job_id(job) == "job4_outlier").is_some());
    let jobs = jobs
        .iter()
        .find(|job| get_job_id(job) == "job3")
        .and_then(|job| job.dimens().get_cluster().cloned())
        .unwrap()
        .into_iter()
        .map(|info| get_job_id(&info.job).clone())
        .collect::<Vec<_>>();
    assert_eq!(jobs, vec!["job3".to_string(), "job2".to_string(), "job1".to_string()]);
}
