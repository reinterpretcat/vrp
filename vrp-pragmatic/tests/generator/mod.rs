//! This module is responsible for the logic which generates problems with specific characteristics.

extern crate proptest;
extern crate uuid;

use proptest::prelude::*;

mod common;
pub use self::common::*;

mod jobs;
pub use self::jobs::*;

prop_compose! {
    fn from_uints(vec: Vec<u64>)(index in 0..vec.len()) -> u64 {
        vec[index]
    }
}

prop_compose! {
    fn from_ints(vec: Vec<i32>)(index in 0..vec.len()) -> i32 {
        vec[index]
    }
}

proptest! {
    #[test]
    fn test_ggg(ggg in delivery_job_prototype(
          simple_job_place_prototype(
            generate_simple_locations(1..100),
            generate_durations(10..20),
            generate_no_tags(),
            generate_multiple_time_windows_fixed(
              "2020-01-01T00:00:00Z",
               vec![from_hours(8), from_hours(16)],
               vec![from_hours(2), from_hours(4)],
               1..3)),
          generate_simple_demand(1..5),
          generate_no_skills())
        ) {
        println!("{:?}", ggg);
    }
}
