use super::*;
use crate::helpers::models::domain::{create_empty_insertion_context, create_simple_insertion_ctx};
use std::cmp::Ordering;

#[test]
fn can_properly_estimate_empty_solution() {
    let empty = create_empty_insertion_context();
    let non_empty = create_simple_insertion_ctx(10., 0);
    let objective = minimize_unassigned_jobs(Arc::new(|_, _| 1.)).unwrap().objective.unwrap();

    let result = objective.total_order(&empty, &non_empty);

    assert_eq!(result, Ordering::Greater);
}
