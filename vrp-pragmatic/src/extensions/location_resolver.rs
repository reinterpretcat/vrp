use crate::format::CoordIndex;
use std::sync::Arc;
use vrp_core::construction::heuristics::LocationResolver;

/// Returns location resolver.
pub(crate) fn get_location_resolver(coord_index: Arc<CoordIndex>) -> LocationResolver {
    // TODO use multidimensional scaling:
    //      * to support location indices
    //      * to have more accurate distance approximation

    LocationResolver {
        func: Arc::new(move |location| coord_index.get_by_idx(location).expect("not implemented").to_lat_lng()),
    }
}
