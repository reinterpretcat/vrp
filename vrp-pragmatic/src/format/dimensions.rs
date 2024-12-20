//! Specifies different properties as extension points on Dimensions type.

use vrp_core::construction::features::BreakPolicy;
use vrp_core::custom_dimension;
use vrp_core::models::common::Dimensions;
use vrp_core::utils::Float;

custom_dimension!(pub VehicleType typeof String);

custom_dimension!(pub ShiftIndex typeof usize);

custom_dimension!(pub TourSize typeof usize);

custom_dimension!(pub PlaceTags typeof Vec<(usize, String)>);

custom_dimension!(pub JobOrder typeof i32);

custom_dimension!(pub JobValue typeof Float);

custom_dimension!(pub JobType typeof String);

custom_dimension!(pub BreakPolicy typeof BreakPolicy);
