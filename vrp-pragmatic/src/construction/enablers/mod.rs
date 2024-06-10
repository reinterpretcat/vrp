//! Provides extension logic for building various VRP features.

use std::sync::Arc;

mod entities;
pub use self::entities::*;

mod location_fallback;
pub use self::location_fallback::*;

mod only_vehicle_activity_cost;
pub use self::only_vehicle_activity_cost::*;
