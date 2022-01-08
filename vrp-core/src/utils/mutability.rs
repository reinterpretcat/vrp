/// # Safety
use std::sync::Arc;

/// Compares pointers from shared objects.
pub fn compare_shared<T: ?Sized>(left: &Arc<T>, right: &Arc<T>) -> bool {
    let left: *const T = left.as_ref();
    let right: *const T = right.as_ref();
    left == right
}

/// Unsafe method which casts immutable reference to mutable reference without any checks.
#[allow(clippy::mut_from_ref)]
pub unsafe fn as_mut<T>(reference: &T) -> &mut T {
    let const_ptr = reference as *const T;
    let mut_ptr = const_ptr as *mut T;
    &mut *mut_ptr
}
