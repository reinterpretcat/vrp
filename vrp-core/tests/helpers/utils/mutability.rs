/// Unsafe method which casts immutable reference to mutable reference without any checks.
///
/// # Safety
/// The caller must ensure that this is safe to do in given context.
#[allow(clippy::mut_from_ref)]
pub unsafe fn as_mut<T>(reference: &T) -> &mut T {
    let const_ptr = reference as *const T;
    let mut_ptr = const_ptr as *mut T;
    &mut *mut_ptr
}
