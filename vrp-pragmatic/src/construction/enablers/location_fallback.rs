use super::*;
use crate::format::{CoordIndex, CustomLocationType, Location as ApiLocation};
use vrp_core::models::common::{Distance, Duration, Location, Profile};
use vrp_core::models::problem::TransportFallback;

/// A transport fallback for unknown location. Returns zero distance/duration for unknown type locations.
pub struct UnknownLocationFallback {
    coord_index: Arc<CoordIndex>,
}

impl UnknownLocationFallback {
    /// Creates a new instance of [`UnknownLocationFallback`];
    pub fn new(coord_index: Arc<CoordIndex>) -> Self {
        Self { coord_index }
    }

    fn get_default_value(&self, from: Location, to: Location) -> f64 {
        let (from, to) = (self.coord_index.get_by_idx(from), self.coord_index.get_by_idx(to));

        match (from, to) {
            (Some(ApiLocation::Custom { r#type: CustomLocationType::Unknown }), _)
            | (_, Some(ApiLocation::Custom { r#type: CustomLocationType::Unknown })) => Duration::default(),
            _ => panic!("fallback is only for locations of custom unknown type"),
        }
    }
}

impl TransportFallback for UnknownLocationFallback {
    fn duration(&self, _: &Profile, from: Location, to: Location) -> Duration {
        self.get_default_value(from, to)
    }

    fn distance(&self, _: &Profile, from: Location, to: Location) -> Distance {
        self.get_default_value(from, to)
    }
}
