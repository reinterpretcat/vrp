use std::cmp::{Ordering, PartialOrd};

macro_rules! compare_float_types {
    ($fn_name_: ident, $type_: ty) => {
        /// Compares floating point numbers.
        pub fn $fn_name_(a: $type_, b: $type_) -> Ordering {
            match (a, b) {
                (x, y) if x.is_nan() && y.is_nan() => Ordering::Equal,
                (x, _) if x.is_nan() => Ordering::Greater,
                (_, y) if y.is_nan() => Ordering::Less,
                (_, _) => a.partial_cmp(&b).unwrap(),
            }
        }
    };
}

compare_float_types! { compare_floats, f64}
compare_float_types! { compare_floats_refs, &f64}
compare_float_types! { compare_floats_f32, f32}
