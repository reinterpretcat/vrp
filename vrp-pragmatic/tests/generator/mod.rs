//! This module is responsible for the logic which generates problems with specific characteristics.

#[cfg(test)]
extern crate proptest;
use proptest::prelude::*;

mod common;
pub use self::common::*;

mod jobs;

prop_compose! {
    fn from_uints(vec: Vec<u64>)(index in 0..vec.len()) -> u64 {
        vec[index]
    }
}

proptest! {
    #[test]
    fn test_ggg(ggg in generate_multiple_time_windows_fixed("2020-01-01T00:00:00Z",
        vec![from_hours(8), from_hours(16)], vec![from_hours(2), from_hours(4)], 1..3)) {
        println!("{:?}", ggg);
    }
}
