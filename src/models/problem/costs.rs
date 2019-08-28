use crate::models::common::{Cost, Distance, Location, Profile, Timestamp};
use crate::models::problem::{Driver, Vehicle};
use crate::models::solution::Activity;

// TODO add default implementation
/// Provides the way to get cost information for specific activities.
pub trait ActivityCost {
    /// Returns cost to perform activity.
    fn cost(
        &self,
        vehicle: &Vehicle,
        driver: &Driver,
        activity: &Activity,
        arrival: Timestamp,
    ) -> Cost;

    /// Returns operation time spent to perform activity.
    fn duration(
        &self,
        vehicle: &Vehicle,
        driver: &Driver,
        activity: &Activity,
        arrival: Timestamp,
    ) -> Cost;
}

/// Provides the way to get routing information for specific locations.
pub trait TransportCost {
    /// Returns transport cost between two locations.
    fn cost(
        &self,
        vehicle: &Vehicle,
        driver: &Driver,
        from: Location,
        to: Location,
        departure: Timestamp,
    ) -> Cost {
        let distance = self.distance(vehicle.profile, from, to, departure);
        let duration = self.duration(vehicle.profile, from, to, departure);

        return distance * (driver.costs.per_distance + vehicle.costs.per_distance)
            + duration * (driver.costs.per_driving_time + vehicle.costs.per_driving_time);
    }

    /// Returns transport time between two locations.
    fn duration(
        &self,
        profile: Profile,
        from: Location,
        to: Location,
        departure: Timestamp,
    ) -> Cost;

    /// Returns transport distance between two locations.
    fn distance(
        &self,
        profile: Profile,
        from: Location,
        to: Location,
        departure: Timestamp,
    ) -> Distance;
}
