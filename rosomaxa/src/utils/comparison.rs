use crate::utils::Float;
use std::cmp::{Ordering, PartialOrd};

macro_rules! compare_float_types {
    ($fn_name_: ident, $type_: ty) => {
        /// Compares floating point numbers.
        #[inline]
        pub fn $fn_name_(a: $type_, b: $type_) -> Ordering {
            match a.partial_cmp(&b) {
                Some(ordering) => ordering,
                None => match (a.is_nan(), b.is_nan()) {
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    _ => Ordering::Equal,
                },
            }
        }
    };
}

compare_float_types! { compare_floats, Float}
compare_float_types! { compare_floats_refs, &Float}

compare_float_types! { compare_floats_f32, f32}
compare_float_types! { compare_floats_f32_refs, &f32}
compare_float_types! { compare_floats_f64, f64}
compare_float_types! { compare_floats_f64_refs, &f64}
