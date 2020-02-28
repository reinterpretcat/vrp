use super::uuid::Uuid;
use super::*;
use crate::json::problem::*;
use crate::json::Location;
use core::ops::Range;

prop_compose! {
    pub fn generate_vehicle(
        amount_proto: impl Strategy<Value = i32>,
        profile_proto: impl Strategy<Value = String>,
        capacity_proto: impl Strategy<Value = Vec<i32>>,
        costs_proto: impl Strategy<Value = VehicleCosts>,
        skills_proto: impl Strategy<Value = Option<Vec<String>>>,
        limits_proto: impl Strategy<Value = Option<VehicleLimits>>,
        shifts_proto: impl Strategy<Value = Vec<VehicleShift>>,
    )
    (amount in amount_proto,
     profile in profile_proto,
     capacity in capacity_proto,
     costs in costs_proto,
     skills in skills_proto,
     limits in limits_proto,
     shifts in shifts_proto)
    -> VehicleType {
        VehicleType {
            id: Uuid::new_v4().to_string(),
            profile,
            costs,
            shifts,
            capacity,
            amount,
            skills,
            limits,
        }
    }
}

prop_compose! {
    pub fn generate_reload(
      locations: impl Strategy<Value = Location>,
      durations: impl Strategy<Value = f64>,
      tags: impl Strategy<Value = Option<String>>,
      time_windows: impl Strategy<Value = Option<Vec<Vec<String>>>>,
    )
    (location in locations,
     duration in durations,
     tag in tags,
     times in time_windows) -> VehicleReload {
        VehicleReload {
          times,
          location,
          duration,
          tag
        }
    }
}

prop_compose! {
    pub fn generate_break(
      locations: impl Strategy<Value = Option<Location>>,
      durations: impl Strategy<Value = f64>,
      time_windows: impl Strategy<Value = Vec<Vec<String>>>,
    )
    (location in locations,
     duration in durations,
     times in time_windows) -> VehicleBreak {
        VehicleBreak {
            times: VehicleBreakTime::TimeWindows(times),
            duration,
            locations: location.map_or(None, |l| Some(vec![l])),
        }
    }
}

/// Generates shifts.
pub fn generate_shifts(
    shift_proto: impl Strategy<Value = VehicleShift>,
    range: Range<usize>,
) -> impl Strategy<Value = Vec<VehicleShift>> {
    prop::collection::vec(shift_proto, range)
}

prop_compose! {
   pub fn generate_shift(
        places_proto: impl Strategy<Value = (VehiclePlace, Option<VehiclePlace>)>,
        breaks_proto: impl Strategy<Value = Option<Vec<VehicleBreak>>>,
        reloads_proto: impl Strategy<Value = Option<Vec<VehicleReload>>>,
    )
      (places in places_proto,
       breaks in breaks_proto,
       reloads in reloads_proto
      ) -> VehicleShift {
        VehicleShift {
          start: places.0,
          end: places.1,
          breaks,
          reloads
        }
    }
}

/// Generates vehicle types.
pub fn generate_vehicles(
    vehicle_proto: impl Strategy<Value = VehicleType>,
    range: Range<usize>,
) -> impl Strategy<Value = Vec<VehicleType>> {
    prop::collection::vec(vehicle_proto, range)
}

prop_compose! {
    /// Generates fleet.
    pub fn generate_fleet(vehicles_proto: impl Strategy<Value = Vec<VehicleType>>,
                          profiles_proto: impl Strategy<Value = Vec<Profile>>)
       (types in vehicles_proto,
        profiles in profiles_proto) -> Fleet {
        Fleet { types, profiles }
    }
}

prop_compose! {
    /// Generates no breaks.
    pub fn generate_no_breaks()(_ in ".*") -> Option<Vec<VehicleBreak>> {
        None
    }
}

prop_compose! {
    /// Generates no reloads.
    pub fn generate_no_reloads()(_ in ".*") -> Option<Vec<VehicleReload>> {
        None
    }
}

prop_compose! {
    /// Generates no limits.
    pub fn generate_no_limits()(_ in ".*") -> Option<VehicleLimits> {
        None
    }
}

prop_compose! {
    pub fn from_costs(vec: Vec<VehicleCosts>)(index in 0..vec.len()) -> VehicleCosts {
        vec[index].clone()
    }
}

prop_compose! {
    /// Generates one dimensional capacity in range.
    pub fn generate_simple_capacity(range: Range<i32>)(capacity in range) -> Vec<i32> {
        vec![capacity]
    }
}
