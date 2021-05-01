use super::*;
use crate::helpers::models::domain::{create_empty_insertion_context, create_simple_insertion_ctx};

#[test]
fn can_properly_estimate_empty_solution() {
    let empty = create_empty_insertion_context();
    let non_empty = create_simple_insertion_ctx(10., 0);
    let objective = TotalUnassignedJobs::new(Arc::new(|_, _, _| 1.));

    let result = objective.total_order(&empty, &non_empty);

    assert_eq!(result, Ordering::Greater);
}
