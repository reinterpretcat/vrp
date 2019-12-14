pub const LATEST_ARRIVAL_KEY: i32 = 1;
pub const WAITING_KEY: i32 = 2;

pub const CURRENT_CAPACITY_KEY: i32 = 11;
pub const MAX_FUTURE_CAPACITY_KEY: i32 = 12;
pub const MAX_PAST_CAPACITY_KEY: i32 = 13;

pub const MAX_DISTANCE_KEY: i32 = 21;
pub const MAX_DURATION_KEY: i32 = 22;

const OP_START_MSG: &str = "Optional start is not yet implemented.";

mod pipeline;
pub use self::pipeline::*;

mod timing;
pub use self::timing::TimingConstraintModule;

mod capacity;
pub use self::capacity::*;

mod traveling;
pub use self::traveling::TravelLimitFunc;
pub use self::traveling::TravelModule;

mod locking;
pub use self::locking::StrictLockingModule;

mod conditional;
pub use self::conditional::ConditionalJobModule;
