use crate::models::common::{Cost, Distance, Location, Profile, Timestamp};
use crate::models::solution::{Activity, Actor};

/// Provides the way to get routing information for specific locations.
pub trait TransportCost {
    /// Returns transport cost between two locations.
    fn cost(&self, actor: &Actor, from: Location, to: Location, departure: Timestamp) -> Cost;

    /// Returns transport time between two locations.
    fn duration(
        &self,
        profile: &Profile,
        from: Location,
        to: Location,
        departure: Timestamp,
    ) -> Cost;

    /// Returns transport distance between two locations.
    fn distance(
        &self,
        profile: &Profile,
        from: Location,
        to: Location,
        departure: Timestamp,
    ) -> Distance;
}
