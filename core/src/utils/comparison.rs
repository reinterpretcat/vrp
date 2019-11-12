use std::cmp::{Ordering, PartialOrd};
use std::sync::Arc;

pub fn compare_floats(a: f64, b: f64) -> Ordering {
    match (a, b) {
        (x, y) if x.is_nan() && y.is_nan() => Ordering::Equal,
        (x, _) if x.is_nan() => Ordering::Greater,
        (_, y) if y.is_nan() => Ordering::Less,
        (_, _) => a.partial_cmp(&b).unwrap(),
    }
}

pub fn compare_shared<T: ?Sized>(left: &Arc<T>, right: &Arc<T>) -> bool {
    let left: *const T = left.as_ref();
    let right: *const T = right.as_ref();
    left == right
}
