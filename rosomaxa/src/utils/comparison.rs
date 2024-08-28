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

compare_float_types! { compare_floats, f64}
compare_float_types! { compare_floats_refs, &f64}
compare_float_types! { compare_floats_f32, f32}
