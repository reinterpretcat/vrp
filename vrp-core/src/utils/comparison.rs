/// Various helpers
use std::sync::Arc;

/// Compares pointers from shared objects.
pub fn compare_shared<T: ?Sized>(left: &Arc<T>, right: &Arc<T>) -> bool {
    let left: *const T = left.as_ref();
    let right: *const T = right.as_ref();
    left == right
}
