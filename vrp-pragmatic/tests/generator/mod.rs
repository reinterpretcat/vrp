//! This module is responsible for the logic which generates problems with specific characteristics.

extern crate proptest;
extern crate uuid;

use proptest::prelude::*;

mod common;
pub use self::common::*;

mod jobs;
pub use self::jobs::*;

mod defaults;
pub use self::defaults::*;

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
            default_job_single_day_time_windows()),
          generate_simple_demand(1..5),
          generate_no_skills())
        ) {
        println!("{:?}", ggg);
    }
}
