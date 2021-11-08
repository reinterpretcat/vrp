//! Solution domain models.

mod route;
pub use self::route::{Activity, Commute, CommuteInfo, Place, Route};

mod registry;
pub use self::registry::Registry;

mod tour;
pub use self::tour::Tour;
