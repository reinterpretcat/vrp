#[cfg(test)]
#[path = "../../tests/unit/utils/types_test.rs"]
mod types_test;

use std::fmt;
use std::fmt::Formatter;
use std::ops::ControlFlow;

/// Alias to a scalar floating type.
///
/// NOTE: Currently, prefer to use `f64` as a default floating type as switching to `f32` leads
/// to precision issues within the solution checker (at least). No clear performance benefits were found.
pub type Float = f64;

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

/// A bit array type of fixed size.
pub struct FixedBitArray<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> Default for FixedBitArray<N> {
    fn default() -> Self {
        Self { data: [0; N] }
    }
}

impl<const N: usize> fmt::Binary for FixedBitArray<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for (count, n) in self.data.iter().enumerate() {
            if count != 0 {
                write!(f, " ")?;
            }

            write!(f, "{:08b}", n.reverse_bits())?;
        }

        Ok(())
    }
}

impl<const N: usize> FixedBitArray<N> {
    /// Sets or unsets bit at given index.
    /// Returns false if index is out of range. Otherwise, returns true.
    pub fn set(&mut self, index: usize, value: bool) -> bool {
        if index >= N * 8 {
            return false;
        }

        let byte_index = index / 8;
        let bit_index = index % 8;

        if value {
            self.data[byte_index] |= 1 << bit_index;
        } else {
            self.data[byte_index] &= !(1 << bit_index);
        }

        true
    }

    /// Gets value at given index.
    /// Always returns false if index is out of range.
    pub fn get(&self, index: usize) -> bool {
        if index >= N * 8 {
            return false;
        }

        let byte_index = index / 8;
        let bit_index = index % 8;

        (self.data[byte_index] & (1 << bit_index)) != 0
    }

    /// Replaces existing value with new value returning an old value.
    /// Always returns false if index is out of range.
    pub fn replace(&mut self, index: usize, new_value: bool) -> bool {
        if index >= N * 8 {
            return false;
        }

        let byte_index = index / 8;
        let bit_index = index % 8;
        let shift = 1 << bit_index;

        let old_value = (self.data[byte_index] & shift) != 0;

        if new_value {
            self.data[byte_index] |= shift;
        } else {
            self.data[byte_index] &= !shift;
        }

        old_value
    }
}
