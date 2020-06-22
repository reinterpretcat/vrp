//! A collection of various utility helpers.

mod comparison;
pub use self::comparison::compare_floats;
pub use self::comparison::compare_shared;

mod iterators;
pub use self::iterators::CollectGroupBy;

mod mutability;
pub use self::mutability::*;

mod parallel;
pub use self::parallel::*;

mod random;
pub use self::random::DefaultRandom;
pub use self::random::Random;

mod time_quota;
pub use self::time_quota::TimeQuota;

mod timing;
pub use self::timing::Timer;
