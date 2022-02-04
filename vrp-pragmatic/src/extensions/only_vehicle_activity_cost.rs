use vrp_core::models::common::{Cost, Timestamp};
use vrp_core::models::problem::{ActivityCost, Actor, SimpleActivityCost};
use vrp_core::models::solution::Activity;

/// Uses costs only for a vehicle ignoring costs of a driver.
#[derive(Default)]
pub struct OnlyVehicleActivityCost {
    inner: SimpleActivityCost,
}

impl ActivityCost for OnlyVehicleActivityCost {
    fn cost(&self, actor: &Actor, activity: &Activity, arrival: Timestamp) -> Cost {
        let waiting = if activity.place.time.start > arrival { activity.place.time.start - arrival } else { 0.0 };
        let service = activity.place.duration;

        waiting * actor.vehicle.costs.per_waiting_time + service * actor.vehicle.costs.per_service_time
    }

    fn estimate_departure(&self, actor: &Actor, activity: &Activity, arrival: Timestamp) -> Timestamp {
        self.inner.estimate_departure(actor, activity, arrival)
    }

    fn estimate_arrival(&self, actor: &Actor, activity: &Activity, departure: Timestamp) -> Timestamp {
        self.inner.estimate_arrival(actor, activity, departure)
    }
}
