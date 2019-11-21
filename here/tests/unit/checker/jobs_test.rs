use crate::checker::jobs::check_stop_has_proper_demand_change;
use crate::checker::models::{StopInfo, TourInfo, VehicleMeta};
use crate::helpers::create_default_vehicle;
use std::sync::Arc;

#[test]
fn can_validate_stop_demand() {
    let tour_info = TourInfo {
        vehicle_meta: VehicleMeta {
            vehicle_id: "my_vehicle_1".to_string(),
            vehicle_type: Arc::new(create_default_vehicle("my_vehicle")),
        },
        stops: vec![],
        schedule: (0.0, 0.0),
    };

    let result = check_stop_has_proper_demand_change(&tour_info);

    assert_eq!(result.err().unwrap(), "TODO");
}
