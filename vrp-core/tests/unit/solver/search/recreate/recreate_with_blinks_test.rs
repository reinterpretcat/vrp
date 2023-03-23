use super::DemandJobSelector;
use crate::construction::heuristics::JobSelector;
use crate::helpers::construction::features::create_simple_demand;
use crate::helpers::construction::heuristics::create_test_insertion_context;
use crate::helpers::models::problem::test_single_with_simple_demand;
use crate::helpers::models::solution::create_test_registry;
use crate::models::common::SingleDimLoad;
use crate::models::problem::Job;

parameterized_test! {can_sort_jobs_by_demand, (demands, is_asc_order, expected), {
        can_sort_jobs_by_demand_impl(demands, is_asc_order, expected);
}}

can_sort_jobs_by_demand! {
        case01: (vec![3, 1, 2], true, vec![1, 2, 3]),
        case02: (vec![-3, 1, 2], true, vec![1, 2, 3]),
        case03: (vec![3, 1, 2], false, vec![3, 2, 1]),
}

fn can_sort_jobs_by_demand_impl(demands: Vec<i32>, is_asc_order: bool, expected: Vec<i32>) {
    let mut insertion_ctx = create_test_insertion_context(create_test_registry());
    demands.into_iter().for_each(|d| {
        insertion_ctx.solution.required.push(Job::Single(test_single_with_simple_demand(create_simple_demand(d))))
    });

    let result = DemandJobSelector::<SingleDimLoad>::new(is_asc_order)
        .select(&mut insertion_ctx)
        .map(|job| DemandJobSelector::<SingleDimLoad>::get_job_demand(&job).unwrap())
        .map(|demand| demand.value)
        .collect::<Vec<i32>>();

    assert_eq!(result, expected);
}
