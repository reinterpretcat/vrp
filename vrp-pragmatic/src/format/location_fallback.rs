use crate::format::{CoordIndex, CustomLocationType, Location as ApiLocation};
use std::sync::Arc;
use vrp_core::models::common::{Distance, Duration, Location, Profile, Timestamp};
use vrp_core::models::problem::TransportFallback;

/// A transport fallback for only a custom unknown location type.
/// Returns zero distance/duration for unknown type locations.
pub struct UnknownLocationFallback {
    coord_index: Arc<CoordIndex>,
}

impl UnknownLocationFallback {
    /// Creates a new instance of [`UnknownLocationFallback`];
    pub fn new(coord_index: Arc<CoordIndex>) -> Self {
        Self { coord_index }
    }

    fn get_default_value(&self, from: Location, to: Location) -> Timestamp {
        let (from, to) = (self.coord_index.get_by_idx(from), self.coord_index.get_by_idx(to));

        match (from, to) {
            (Some(ApiLocation::Custom { r#type: CustomLocationType::Unknown }), _)
            | (_, Some(ApiLocation::Custom { r#type: CustomLocationType::Unknown })) => Timestamp::default(),
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
