//! This code is used by multiple examples.

use vrp_core::prelude::*;

/// Gets a routing matrix for 5 unique locations.
pub fn define_routing_data() -> GenericResult<impl TransportCost> {
    // define distance/duration matrix (use the same data for both)
    // as we have five locations, we need to define 5x5 matrix, flatten to 1 dimension:
    #[rustfmt::skip]
    let routing_data = vec![
    //  0     1     2     3     4
        0.,  500., 520., 530., 540.,  // 0
        500.,  0.,  30.,  40.,  50.,  // 1
        520., 30.,   0.,  20.,  25.,  // 2
        530., 40.,  20.,   0.,  15.,  // 3
        540., 50.,  25.,  15.,   0.   // 4
    ];
    let (durations, distances) = (routing_data.clone(), routing_data);

    // `SimpleTransportCost` provides a simple way to use single routing matrix for any vehicle in the problem
    SimpleTransportCost::new(durations, distances)
}
