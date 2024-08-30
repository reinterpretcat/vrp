//! Specifies different properties as extension points on Dimensions type.

use vrp_core::construction::features::BreakPolicy;
use vrp_core::custom_dimension;
use vrp_core::models::common::Dimensions;
use vrp_core::utils::Float;

custom_dimension!(VehicleType typeof String);

custom_dimension!(ShiftIndex typeof usize);

custom_dimension!(TourSize typeof usize);

custom_dimension!(PlaceTags typeof Vec<(usize, String)>);

custom_dimension!(JobOrder typeof i32);

custom_dimension!(JobValue typeof Float);

custom_dimension!(JobType typeof String);

custom_dimension!(BreakPolicy typeof BreakPolicy);
