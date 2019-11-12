pub mod construction;
pub mod models;
pub mod objectives;
pub mod refinement;
pub mod utils;

// See https://stackoverflow.com/questions/30429801/whats-the-most-idiomatic-way-to-test-two-options-for-equality-when-they-contain
macro_rules! cmp_eq_option {
    ($left:expr, $right:expr) => {{
        match (&$left, &$right) {
            (Some(left_val), Some(right_val)) => *left_val == *right_val,
            (None, None) => true,
            _ => false,
        }
    }};
}

#[macro_use]
pub mod macros;
