use super::*;
use crate::helpers::construction::features::create_simple_demand;
use crate::helpers::models::problem::TestSingleBuilder;

#[test]
fn can_sort_jobs_by_largest_demand_first() {
    let mut jobs = [1, 3, 2]
        .into_iter()
        .map(|demand| TestSingleBuilder::default().demand(create_simple_demand(demand)).build_as_job_ref())
        .collect::<Vec<_>>();

    sort_jobs_by_demand(&mut jobs);

    assert_eq!(jobs.iter().map(get_job_demand).collect::<Vec<_>>(), vec![3, 2, 1]);
}
