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
