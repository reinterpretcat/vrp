use super::DemandJobSelector;
use crate::construction::heuristics::JobSelector;
use crate::helpers::construction::features::create_simple_demand;
use crate::helpers::construction::heuristics::InsertionContextBuilder;
use crate::helpers::models::problem::SingleBuilder;
use crate::models::common::SingleDimLoad;
use crate::models::CoreStateKeys;

parameterized_test! {can_sort_jobs_by_demand, (demands, is_asc_order, expected), {
        can_sort_jobs_by_demand_impl(demands, is_asc_order, expected);
}}

can_sort_jobs_by_demand! {
        case01: (vec![3, 1, 2], true, vec![1, 2, 3]),
        case02: (vec![-3, 1, 2], true, vec![1, 2, 3]),
        case03: (vec![3, 1, 2], false, vec![3, 2, 1]),
}

fn can_sort_jobs_by_demand_impl(demands: Vec<i32>, is_asc_order: bool, expected: Vec<i32>) {
    let mut insertion_ctx = InsertionContextBuilder::default().build();
    let demand_key = insertion_ctx.problem.extras.get_capacity_keys().unwrap().dimen_keys.activity_demand;
    demands.into_iter().for_each(|d| {
        insertion_ctx
            .solution
            .required
            .push(SingleBuilder::default().demand(demand_key, create_simple_demand(d)).build_as_job_ref())
    });
    let selector = DemandJobSelector::<SingleDimLoad>::new(is_asc_order);

    selector.prepare(&mut insertion_ctx);
    let result = selector
        .select(&insertion_ctx)
        .map(|job| selector.get_job_demand(job, demand_key).unwrap())
        .map(|demand| demand.value)
        .collect::<Vec<i32>>();

    assert_eq!(result, expected);
}
