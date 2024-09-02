use crate::models::common::{Cost, Duration, Timestamp};
use crate::models::problem::{ActivityCost, SimpleActivityCost};
use crate::models::solution::Activity;
use crate::models::solution::Route;

/// Uses costs only for a vehicle ignoring costs of a driver.
#[derive(Default)]
pub struct OnlyVehicleActivityCost {
    inner: SimpleActivityCost,
}

impl ActivityCost for OnlyVehicleActivityCost {
    fn cost(&self, route: &Route, activity: &Activity, arrival: Timestamp) -> Cost {
        let actor = route.actor.as_ref();

        let waiting =
            if activity.place.time.start > arrival { activity.place.time.start - arrival } else { Duration::default() };
        let service = activity.place.duration;

        waiting as f64 * actor.vehicle.costs.per_waiting_time + service as f64 * actor.vehicle.costs.per_service_time
    }

    fn estimate_departure(&self, route: &Route, activity: &Activity, arrival: Timestamp) -> Timestamp {
        self.inner.estimate_departure(route, activity, arrival)
    }

    fn estimate_arrival(&self, route: &Route, activity: &Activity, departure: Timestamp) -> Timestamp {
        self.inner.estimate_arrival(route, activity, departure)
    }
}
