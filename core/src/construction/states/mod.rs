const OP_START_MSG: &str = "Optional start is not yet implemented.";

mod route;
pub use self::route::create_end_activity;
pub use self::route::create_start_activity;
pub use self::route::RouteContext;
pub use self::route::RouteState;

mod models;
pub use self::models::ActivityContext;
pub use self::models::InsertionContext;
pub use self::models::InsertionFailure;
pub use self::models::InsertionProgress;
pub use self::models::InsertionResult;
pub use self::models::InsertionSuccess;
pub use self::models::SolutionContext;
