//! Various built-in constraints applied to customers and vehicles/drivers.
//!
//!
//! ## Constraint
//!
//! Constraint represents some limitation which should be applied to solution. A good examples:
//!
//! - **time**: customer can be visited only in specific time window, e.g. from 9am till 11am
//! - **capacity**: there is a fleet and multiple customers with total demand exceeding capacity
//!   of one vehicle from the fleet.
//! - **shift-time**: vehicle or driver cannot operate more than specific amount of time.
//!
//! Typically, VRP can have many of such constraints applied to its solution.
//!
//!
//! ## Design
//!
//! There are multiple types of constraints described below in details. In common, all of them try
//! to identify insertion possibility or cost of given customer known as `Job` into given route.
//!
//!
//! ### Constraint characteristics
//! Each constraint has two characteristic:
//!
//! - **hard or soft**: this characteristic defines what should happen when constraint is violated.
//!     When hard constraint is violated, it means that given customer cannot be served with given
//!     route. In contrast to this, soft constraint allows insertion but applies some penalty to
//!     make violation less attractive.
//!
//! - **route or activity**: this characteristic defines on which level constrain is executed.
//!     As a heuristic algorithm is based on insertion heuristic, insertion of one customer is
//!     evaluated on each leg of one route. When it does not make sense, the route constraint
//!     can be used as it is called only once to check whether customer can be inserted in given
//!     route.
//!
//!
//! ### Constraint module
//!
//! Sometimes you might need multiple constraints with different characteristics to implement some
//! aspect of VRP variation. This is where `ConstraintModule` supposed to be used: it allows you
//! to group multiple constraints together keeping implementation details hidden outside of module.
//! Additionally, `ConstraintModule` provides the way to share some state between insertions.
//! This is really important as allows you to avoid unneeded computations.
//!
//!
//! ### Sharing state
//!
//! You can share some state using `RouteState` object which is part of `RouteContext`. It is
//! read-only during insertion evaluation in all constraint types, but it is mutable via `ConstraintModule`
//! methods once best insertion is identified.
//!
//!
//! ### Constraint pipeline
//!
//! All constraint modules are organized inside one `ConstraintPipeline` which specifies the order
//! of their execution.

/// A key which tracks latest arrival.
pub const LATEST_ARRIVAL_KEY: i32 = 1;
/// A key which tracks waiting time.
pub const WAITING_KEY: i32 = 2;
/// A key which tracks total distance.
pub const TOTAL_DISTANCE_KEY: i32 = 3;
/// A key which tracks total duration.
pub const TOTAL_DURATION_KEY: i32 = 4;
/// A key which tracks global duration limit.
pub const LIMIT_DURATION_KEY: i32 = 5;

/// A key which tracks current vehicle capacity.
pub const CURRENT_CAPACITY_KEY: i32 = 11;
/// A key which tracks maximum vehicle capacity ahead in route.
pub const MAX_FUTURE_CAPACITY_KEY: i32 = 12;
/// A key which tracks maximum capacity backward in route.
pub const MAX_PAST_CAPACITY_KEY: i32 = 13;
/// A key which tracks reload intervals.
pub const RELOAD_INTERVALS_KEY: i32 = 14;
/// A key which tracks max load in tour.
pub const MAX_LOAD_KEY: i32 = 15;

#[allow(clippy::unnecessary_wraps)]
fn fail(code: i32) -> Option<ActivityConstraintViolation> {
    Some(ActivityConstraintViolation { code, stopped: true })
}

#[allow(clippy::unnecessary_wraps)]
fn stop(code: i32) -> Option<ActivityConstraintViolation> {
    Some(ActivityConstraintViolation { code, stopped: false })
}

#[allow(clippy::unnecessary_wraps)]
fn success() -> Option<ActivityConstraintViolation> {
    None
}

mod pipeline;
pub use self::pipeline::*;

mod transport;
pub use self::transport::*;

mod capacity;
pub use self::capacity::*;

mod locking;
pub use self::locking::*;

mod tour_size;
pub use self::tour_size::*;

mod conditional;
pub use self::conditional::*;

mod fleet_usage;
pub use self::fleet_usage::*;

mod travel_limit;
pub use self::travel_limit::*;
