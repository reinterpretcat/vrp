use crate::models::common::{Cost, Timestamp};
use crate::models::solution::{Activity, Actor};

// TODO add default implementation
/// Provides the way to get cost information for specific activities.
pub trait ActivityCost {
    /// Returns cost to perform activity.
    fn cost(&self, actor: &Actor, activity: &Activity, arrival: Timestamp) -> Cost;

    /// Returns operation time spent to perform activity.
    fn duration(&self, actor: &Actor, activity: &Activity, arrival: Timestamp) -> Cost;
}
