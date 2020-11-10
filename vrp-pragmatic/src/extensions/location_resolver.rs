use crate::format::CoordIndex;
use std::sync::Arc;
use vrp_core::construction::heuristics::LocationResolver;

/// Returns location resolver.
pub(crate) fn get_location_resolver(coord_index: Arc<CoordIndex>) -> LocationResolver {
    // TODO use multidimensional scaling:
    //      * to support location indices
    //      * to have more accurate distance approximation

    let (_, has_indices) = coord_index.get_used_types();

    if has_indices {
        LocationResolver { func: Arc::new(|_| (0., 0.)) }
    } else {
        LocationResolver {
            func: Arc::new(move |location| coord_index.get_by_idx(location).expect("not implemented").to_lat_lng()),
        }
    }
}
