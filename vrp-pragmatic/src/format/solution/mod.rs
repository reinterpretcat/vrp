//! Specifies logic to create a "pragmatic" solution and write it into json format.

mod model;
pub use self::model::*;

pub(crate) mod activity_matcher;

mod geo_serializer;
pub use self::geo_serializer::*;

mod initial_reader;
pub use self::initial_reader::read_init_solution;

mod extensions;

mod problem_writer;
pub use self::problem_writer::{create_solution, write_pragmatic, PragmaticOutputType};

use super::*;

fn map_code_reason(code: i32) -> (&'static str, &'static str) {
    match code {
        SKILL_CONSTRAINT_CODE => ("SKILL_CONSTRAINT", "cannot serve required skill"),
        TIME_CONSTRAINT_CODE => ("TIME_WINDOW_CONSTRAINT", "cannot be visited within time window"),
        CAPACITY_CONSTRAINT_CODE => ("CAPACITY_CONSTRAINT", "does not fit into any vehicle due to capacity"),
        REACHABLE_CONSTRAINT_CODE => ("REACHABLE_CONSTRAINT", "location unreachable"),
        DISTANCE_LIMIT_CONSTRAINT_CODE => {
            ("MAX_DISTANCE_CONSTRAINT", "cannot be assigned due to max distance constraint of vehicle")
        }
        DURATION_LIMIT_CONSTRAINT_CODE => {
            ("MAX_DURATION_CONSTRAINT", "cannot be assigned due to max duration constraint of vehicle")
        }
        BREAK_CONSTRAINT_CODE => ("BREAK_CONSTRAINT", "break is not assignable"),
        LOCKING_CONSTRAINT_CODE => ("LOCKING_CONSTRAINT", "cannot be served due to relation lock"),
        AREA_CONSTRAINT_CODE => ("AREA_CONSTRAINT", "cannot be assigned due to area constraint"),
        DISPATCH_CONSTRAINT_CODE => ("DISPATCH_CONSTRAINT", "cannot be assigned due to vehicle dispatch"),
        TOUR_SIZE_CONSTRAINT_CODE => {
            ("TOUR_SIZE_CONSTRAINT", "cannot be assigned due to tour size constraint of vehicle")
        }
        TOUR_ORDER_CONSTRAINT_CODE => ("TOUR_ORDER_CONSTRAINT", "cannot be assigned due to tour order constraint"),
        GROUP_CONSTRAINT_CODE => ("GROUP_CONSTRAINT", "cannot be assigned due to group constraint"),
        COMPATIBILITY_CONSTRAINT_CODE => {
            ("COMPATIBILITY_CONSTRAINT", "cannot be assigned due to compatibility constraint")
        }
        RELOAD_RESOURCE_CONSTRAINT_CODE => {
            ("RELOAD_RESOURCE_CONSTRAINT", "cannot be assigned due to reload resource constraint")
        }
        _ => ("NO_REASON_FOUND", "unknown"),
    }
}

fn map_reason_code(reason: &str) -> i32 {
    match reason {
        "SKILL_CONSTRAINT" => SKILL_CONSTRAINT_CODE,
        "TIME_WINDOW_CONSTRAINT" => TIME_CONSTRAINT_CODE,
        "CAPACITY_CONSTRAINT" => CAPACITY_CONSTRAINT_CODE,
        "REACHABLE_CONSTRAINT" => REACHABLE_CONSTRAINT_CODE,
        "MAX_DISTANCE_CONSTRAINT" => DISTANCE_LIMIT_CONSTRAINT_CODE,
        "MAX_DURATION_CONSTRAINT" => DURATION_LIMIT_CONSTRAINT_CODE,
        "BREAK_CONSTRAINT" => BREAK_CONSTRAINT_CODE,
        "LOCKING_CONSTRAINT" => LOCKING_CONSTRAINT_CODE,
        "AREA_CONSTRAINT" => AREA_CONSTRAINT_CODE,
        "DISPATCH_CONSTRAINT" => DISPATCH_CONSTRAINT_CODE,
        "TOUR_SIZE_CONSTRAINT" => TOUR_SIZE_CONSTRAINT_CODE,
        "TOUR_ORDER_CONSTRAINT" => TOUR_ORDER_CONSTRAINT_CODE,
        "GROUP_CONSTRAINT" => GROUP_CONSTRAINT_CODE,
        "COMPATIBILITY_CONSTRAINT" => COMPATIBILITY_CONSTRAINT_CODE,
        "RELOAD_RESOURCE_CONSTRAINT" => RELOAD_RESOURCE_CONSTRAINT_CODE,
        _ => -1,
    }
}
