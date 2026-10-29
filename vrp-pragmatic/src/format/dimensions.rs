//! Specifies different properties as extension points on Dimensions type.

use vrp_core::construction::features::BreakPolicy;
use vrp_core::custom_dimension;
use vrp_core::models::common::Dimensions;
use vrp_core::models::problem::Single;
use vrp_core::utils::Float;

custom_dimension!(pub VehicleType typeof String);

custom_dimension!(pub ShiftIndex typeof usize);

custom_dimension!(pub TourSize typeof usize);

custom_dimension!(pub MinTourSize typeof usize);

custom_dimension!(pub PlaceTags typeof Vec<(usize, String)>);

custom_dimension!(pub JobOrder typeof i32);

custom_dimension!(pub JobValue typeof Float);

custom_dimension!(pub ProductionValue typeof Float);

custom_dimension!(pub JobDueDate typeof Float);

custom_dimension!(pub JobType typeof String);

custom_dimension!(pub BreakPolicy typeof BreakPolicy);

/// Whether a job's activity is a customer visit. A break, a reload or a recharge is on the tour as
/// an activity, but it is not one, and three readers ask: `tourSize` / `minTourSize` leave them out
/// of the count, the shift's appointment bounds do not govern when they may happen, and the
/// solution writer must not report one as held back by a bound the solver never applied to it.
///
/// ⚠️ The wiring in `problem_reader` and `get_earliest_first` in the solution writer must answer
/// this the same way, or a tour reports a wait that never happened.
pub fn is_stop(single: &Single) -> bool {
    !matches!(single.dimens.get_job_type().map(String::as_str), Some("break" | "reload" | "recharge"))
}
