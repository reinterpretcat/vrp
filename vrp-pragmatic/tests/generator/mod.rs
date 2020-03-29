//! This module is responsible for the logic which generates problems with specific characteristics.

extern crate proptest;
extern crate uuid;

use proptest::prelude::*;

mod common;
pub use self::common::*;

mod jobs;
pub use self::jobs::*;

mod relations;
pub use self::relations::*;

mod defaults;
pub use self::defaults::*;

mod vehicles;
pub use self::vehicles::*;

prop_compose! {
    fn from_uints(vec: Vec<u64>)(index in 0..vec.len()) -> u64 {
        vec[index]
    }
}

prop_compose! {
    fn from_usize(vec: Vec<usize>)(index in 0..vec.len()) -> usize {
        vec[index]
    }
}

prop_compose! {
    fn from_strings(vec: Vec<String>)(index in 0..vec.len()) -> String {
        vec[index].clone()
    }
}
