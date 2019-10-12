mod solomon;
pub use self::solomon::SolomonBuilder;

use crate::helpers::get_test_resource;
use crate::models::Problem;
use crate::streams::input::text::SolomonProblem;

pub fn create_c101_25_problem() -> Problem {
    get_test_resource("data/solomon/C101.25.txt").unwrap().parse_solomon().unwrap()
}
