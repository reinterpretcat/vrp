use std::ops::ControlFlow;

/// Unwraps value from inner state.
pub trait UnwrapValue {
    /// A value type.
    type Value;

    /// Unwraps value from the type.
    fn unwrap_value(self) -> Self::Value;
}

impl<T> UnwrapValue for ControlFlow<T, T> {
    type Value = T;

    fn unwrap_value(self) -> Self::Value {
        match self {
            ControlFlow::Continue(value) => value,
            ControlFlow::Break(value) => value,
        }
    }
}

/// Returns a short name of a type.
pub fn short_type_name<T: ?Sized>() -> &'static str {
    let name = std::any::type_name::<T>();

    name.rsplit_once(':').map(|(_, name)| name).unwrap_or(name)
}
