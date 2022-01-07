use std::cmp::{Ordering, PartialOrd};

/// Compares floats.
pub fn compare_floats(a: f64, b: f64) -> Ordering {
    match (a, b) {
        (x, y) if x.is_nan() && y.is_nan() => Ordering::Equal,
        (x, _) if x.is_nan() => Ordering::Greater,
        (_, y) if y.is_nan() => Ordering::Less,
        (_, _) => a.partial_cmp(&b).unwrap(),
    }
}

/// Unwraps result type.
pub fn unwrap_from_result<T>(result: Result<T, T>) -> T {
    match result {
        Ok(result) => result,
        Err(result) => result,
    }
}
