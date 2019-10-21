use crate::construction::heuristics::JobSelector;
use crate::helpers::construction::constraints::create_simple_demand;
use crate::helpers::construction::states::create_test_insertion_context;
use crate::helpers::models::problem::test_single_job_with_simple_demand;
use crate::helpers::models::solution::create_test_registry;
use crate::refinement::recreate::recreate_with_blinks::DemandJobSelector;
use std::sync::Arc;

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
        insertion_ctx.solution.required.push(Arc::new(test_single_job_with_simple_demand(create_simple_demand(d))))
    });

    let result = DemandJobSelector::<i32>::new(is_asc_order)
        .select(&mut insertion_ctx)
        .map(|job| DemandJobSelector::<i32>::get_job_demand(&job).unwrap())
        .collect::<Vec<i32>>();

    assert_eq!(result, expected);
}
