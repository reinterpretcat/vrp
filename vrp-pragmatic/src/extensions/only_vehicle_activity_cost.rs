use vrp_core::models::common::{Cost, Timestamp};
use vrp_core::models::problem::{ActivityCost, Actor};
use vrp_core::models::solution::Activity;

/// Uses costs only for vehicle ignoring costs of driver.
pub struct OnlyVehicleActivityCost {}

impl ActivityCost for OnlyVehicleActivityCost {
    fn cost(&self, actor: &Actor, activity: &Activity, arrival: Timestamp) -> Cost {
        let waiting = if activity.place.time.start > arrival { activity.place.time.start - arrival } else { 0.0 };
        let service = self.duration(actor, activity, arrival);

        waiting * actor.vehicle.costs.per_waiting_time + service * actor.vehicle.costs.per_service_time
    }
}

impl Default for OnlyVehicleActivityCost {
    fn default() -> Self {
        Self {}
    }
}
